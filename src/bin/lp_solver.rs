use preference_splitting::EDGE_COST_DIMENSION;

use glpk_sys::*;
use structopt::StructOpt;

use std::error::Error;
use std::ffi::CString;
use std::os::raw::c_int;

const GLP_MAX: c_int = 2; // maximisation
const GLP_LO: c_int = 2; // variable with lower bound
const GLP_DB: c_int = 4; // double-bounded variable
const GLP_CV: c_int = 1; // continuous variable
const GLP_FX: c_int = 5; // fixed variable
const GLP_ON: c_int = 1; // enable something
const GLP_MSG_OFF: c_int = 0; // no output
const GLP_DUALP: c_int = 2; // use dual; if it fails, use primal
const GLP_OPT: c_int = 5; // solution is optimal
const GLP_FEAS: c_int = 2; // solution is feasible

struct Lp {
    lp: *mut glp_prob,
    dim: c_int,
    delta_col: c_int,
}

impl Lp {
    fn new(dim: c_int) -> Lp {
        let (lp, delta_col) = unsafe {
            let lp = glp_create_prob();
            glp_set_obj_dir(lp, GLP_MAX);
            let delta_col = Self::init_variables(lp, dim);

            (lp, delta_col)
        };
        let mut lp = Self { lp, dim, delta_col };
        unsafe {
            lp.add_sum_of_alpha_eq_one();
        }
        lp
    }

    unsafe fn init_variables(lp: *mut glp_prob, dim: c_int) -> c_int {
        glp_add_cols(lp, dim);
        for i in 0..dim {
            let name =
                CString::new(format!("alpha_{}", i)).expect("Column name could not be created");
            glp_set_col_bnds(lp, i + 1, GLP_DB, 0.0, 1.0);
            glp_set_col_kind(lp, i + 1, GLP_CV);
            glp_set_obj_coef(lp, i + 1, 0.0);
            glp_set_col_name(lp, i + 1, name.as_ptr());
        }

        let delta_col = glp_add_cols(lp, 1);

        let name = CString::new("delta").expect("Delta col name could not be created");

        glp_set_col_bnds(lp, delta_col, GLP_LO, 0.0, 0.0);
        glp_set_col_kind(lp, delta_col, GLP_CV);
        glp_set_obj_coef(lp, delta_col, 1.0);
        glp_set_col_name(lp, delta_col, name.as_ptr());
        delta_col
    }

    unsafe fn add_sum_of_alpha_eq_one(&mut self) {
        let row = glp_add_rows(self.lp, 1);
        let indices: Vec<_> = (0..=self.dim).collect();
        let values = vec![1.0; self.dim as usize + 1];

        glp_set_row_bnds(self.lp, row, GLP_FX, 1.0, 1.0);
        glp_set_mat_row(self.lp, row, self.dim, indices.as_ptr(), values.as_ptr());
    }

    fn add_constraint(&mut self, coeff: &[f64]) {
        if !coeff.len() != self.dim as usize {
            panic!(format!(
                "got wrong number of coefficients ({} instead of {})",
                coeff.len(),
                self.dim
            ));
        }
        unsafe {
            let row = glp_add_rows(self.lp, 1);
            // indices for alpha cols + index of delta col
            let indices: Vec<_> = (0..=self.dim)
                .chain(std::iter::once(self.delta_col))
                .collect();

            // values for alpha cols + value of delta col
            let values: Vec<_> = coeff.iter().copied().chain(std::iter::once(-1.0)).collect();

            glp_set_row_bnds(self.lp, row, GLP_LO, 0.0, 0.0);
            glp_set_mat_row(
                self.lp,
                row,
                self.dim + 1,
                indices.as_ptr(),
                values.as_ptr(),
            );
        }
    }

    fn solve(&mut self) -> Result<[f64; EDGE_COST_DIMENSION + 1], LpError> {
        unsafe {
            let mut params = glp_smcp::default();
            glp_init_smcp(&mut params);
            params.presolve = GLP_ON;
            params.msg_lev = GLP_MSG_OFF;
            params.meth = GLP_DUALP;
            let status = glp_simplex(self.lp, &params);
            if status == 0 {
                let status = glp_get_status(self.lp);
                if !(status == GLP_OPT || status == GLP_FEAS) {
                    return Err(LpError::Infeasible);
                }
            } else {
                return Err(LpError::Infeasible);
            }
            let mut result = [0.0; EDGE_COST_DIMENSION + 1];
            for i in 0..self.dim {
                result[i as usize] = glp_get_col_prim(self.lp, i + 1);
            }

            *result.last_mut().unwrap() = glp_get_col_prim(self.lp, self.delta_col);
            Ok(result)
        }
    }
}

impl Drop for Lp {
    fn drop(&mut self) {
        unsafe {
            glp_delete_prob(self.lp);
        }
    }
}

#[derive(StructOpt)]
struct Opts {
    dim: c_int,
}

const F64_SIZE: usize = std::mem::size_of::<f64>();
const BUFFER_SIZE: usize = F64_SIZE * EDGE_COST_DIMENSION;
const OUTPUT_BUFFER_SIZE: usize = F64_SIZE * (EDGE_COST_DIMENSION + 1);

fn main() -> Result<(), Box<dyn Error>> {
    use std::io::{BufReader, BufWriter, Read, Write};
    let Opts { dim } = Opts::from_args();

    let mut buffer = [0u8; BUFFER_SIZE];
    let stdin = std::io::stdin();
    let stdin = stdin.lock();
    let mut reader = BufReader::new(stdin);

    let stdout = std::io::stdout();
    let stdout = stdout.lock();
    let mut writer = BufWriter::new(stdout);

    loop {
        let mut lp = Lp::new(dim);

        while let Ok(()) = reader.read_exact(&mut buffer) {
            let mut byte_buffer = [0u8; F64_SIZE];
            let values: Vec<_> = buffer
                .chunks_exact(F64_SIZE)
                .map(|slice| {
                    byte_buffer.copy_from_slice(slice);
                    f64::from_ne_bytes(byte_buffer)
                })
                .collect();

            lp.add_constraint(&values);
            match lp.solve() {
                Ok(results) => {
                    let mut output = [0u8; OUTPUT_BUFFER_SIZE];

                    results
                        .iter()
                        .zip(output.chunks_exact_mut(F64_SIZE))
                        .for_each(|(f, slice)| {
                            slice.copy_from_slice(&f.to_ne_bytes());
                        });

                    writer.write_all(&output).unwrap();
                }
                Err(LpError::Infeasible) => println!("infeasible"),
            }
        }
    }
}

#[derive(Debug, Clone)]
enum LpError {
    Infeasible,
}
