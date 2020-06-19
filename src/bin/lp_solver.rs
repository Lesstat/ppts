use preference_splitting::lp::{BUFFER_SIZE, F64_SIZE, OUTPUT_BUFFER_SIZE};
use preference_splitting::EDGE_COST_DIMENSION;

use glpk_sys::*;

use std::error::Error;
use std::ffi::CString;
use std::io::{BufReader, BufWriter, Read, Write};
use std::os::raw::c_int;

const DIM: c_int = EDGE_COST_DIMENSION as c_int;
const GLP_MAX: c_int = 2; // maximisation
const GLP_LO: c_int = 2; // variable with lower bound
const GLP_DB: c_int = 4; // double-bounded variable
const GLP_CV: c_int = 1; // continuous variable
const GLP_FR: c_int = 1; // free (unbounded) variable
const GLP_FX: c_int = 5; // fixed variable
const GLP_ON: c_int = 1; // enable something
const GLP_OFF: c_int = 0; // disable something
const GLP_MSG_OFF: c_int = 0; // no output
const GLP_OPT: c_int = 5; // solution is optimal
const GLP_FEAS: c_int = 2; // solution is feasible

// const GLP_DUALP: c_int = 2; // use dual; if it fails, use primal

struct Lp {
    lp: *mut glp_prob,
    delta_col: c_int,
    counter: usize,
}

impl Lp {
    fn new(counter: usize) -> Lp {
        let (lp, delta_col) = unsafe {
            let lp = glp_create_prob();
            glp_set_obj_dir(lp, GLP_MAX);
            let delta_col = Self::init_variables(lp);

            (lp, delta_col)
        };
        let mut lp = Self {
            lp,
            delta_col,
            counter,
        };
        unsafe {
            lp.add_sum_of_alpha_eq_one();
        }
        lp
    }

    unsafe fn init_variables(lp: *mut glp_prob) -> c_int {
        glp_add_cols(lp, DIM);
        for i in 0..DIM {
            let name =
                CString::new(format!("alpha_{}", i)).expect("Column name could not be created");
            glp_set_col_bnds(lp, i + 1, GLP_DB, 0.0, 1.0);
            glp_set_col_kind(lp, i + 1, GLP_CV);
            glp_set_obj_coef(lp, i + 1, 0.0);
            glp_set_col_name(lp, i + 1, name.as_ptr());
        }

        let delta_col = glp_add_cols(lp, 1);

        let name = CString::new("delta").expect("Delta col name could not be created");

        glp_set_col_bnds(lp, delta_col, GLP_FR, 0.0, 0.0);
        glp_set_col_kind(lp, delta_col, GLP_CV);
        glp_set_obj_coef(lp, delta_col, 1.0);
        glp_set_col_name(lp, delta_col, name.as_ptr());
        delta_col
    }

    unsafe fn add_sum_of_alpha_eq_one(&mut self) {
        let row = glp_add_rows(self.lp, 1);
        let indices: Vec<_> = (0..=DIM).collect();
        let values = vec![1.0; EDGE_COST_DIMENSION as usize + 1];

        glp_set_row_bnds(self.lp, row, GLP_FX, 1.0, 1.0);
        glp_set_mat_row(self.lp, row, DIM, indices.as_ptr(), values.as_ptr());
    }

    fn add_constraint(&mut self, coeff: &[f64]) {
        if coeff.len() != EDGE_COST_DIMENSION as usize {
            panic!(format!(
                "got wrong number of coefficients ({} instead of {})",
                coeff.len(),
                EDGE_COST_DIMENSION
            ));
        }
        unsafe {
            let row = glp_add_rows(self.lp, 1);
            // leading 0 + indices for alpha cols + index of delta col
            let indices: Vec<_> = (0..=DIM).chain(std::iter::once(self.delta_col)).collect();

            // leading 0 + values for alpha cols + value of delta col
            let values: Vec<_> = std::iter::once(0.0)
                .chain(coeff.iter().copied())
                .chain(std::iter::once(-1.0))
                .collect();

            // 0 <= cost(alpha, p_alpha) - cost(alpha, p_trajectory) - delta

            glp_set_row_bnds(self.lp, row, GLP_LO, 0.0, 0.0);
            glp_set_mat_row(self.lp, row, DIM + 1, indices.as_ptr(), values.as_ptr());
        }
    }

    fn solve(&mut self) -> Result<[f64; EDGE_COST_DIMENSION + 1], LpError> {
        unsafe {
            let mut params = glp_smcp::default();
            glp_init_smcp(&mut params);
            params.presolve = GLP_ON;
            params.msg_lev = GLP_MSG_OFF;
            // params.meth = GLP_DUALP;

            // let filename = CString::new(format!("/tmp/lps/my-{}.lp", self.counter)).unwrap();
            // let file_stat = glp_write_lp(self.lp, std::ptr::null(), filename.as_ptr());
            // if file_stat != 0 {
            //     panic!("could not write file");
            // }
            // self.counter += 1;

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
            for i in 0..DIM {
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

fn main() -> Result<(), Box<dyn Error>> {
    unsafe {
        glp_term_out(GLP_OFF);
    }

    let mut buffer = [0u8; BUFFER_SIZE];
    let stdin = std::io::stdin();
    let stdin = stdin.lock();
    let mut reader = BufReader::new(stdin);

    let stdout = std::io::stdout();
    let stdout = stdout.lock();
    let mut writer = BufWriter::new(stdout);

    let mut control_byte = [0u8; 1];
    let mut lp = Lp::new(0);
    loop {
        if reader.read_exact(&mut control_byte).is_err() {
            return Ok(());
        }

        match control_byte[0] {
            0 => lp = Lp::new(lp.counter),
            1 => {
                reader.read_exact(&mut buffer)?;

                let mut byte_buffer = [0u8; F64_SIZE];
                let values: Vec<_> = buffer
                    .chunks_exact(F64_SIZE)
                    .map(|slice| {
                        byte_buffer.copy_from_slice(slice);
                        f64::from_ne_bytes(byte_buffer)
                    })
                    .collect();

                lp.add_constraint(&values);
            }
            2 => {
                match lp.solve() {
                    Ok(results) => {
                        let mut output = [0u8; OUTPUT_BUFFER_SIZE];

                        results
                            .iter()
                            .zip(output.chunks_exact_mut(F64_SIZE))
                            .for_each(|(f, slice)| {
                                slice.copy_from_slice(&f.to_ne_bytes());
                            });

                        control_byte[0] = 0;
                        writer.write_all(&control_byte)?;
                        writer.write_all(&output)?;
                    }
                    Err(LpError::Infeasible) => {
                        control_byte[0] = 1;
                        writer.write_all(&control_byte)?;
                    }
                }
                writer.flush()?;
            }
            x => panic!(format!("Unknown control byte received on lp side: {}", x)),
        }
    }
}

#[derive(Debug, Clone)]
enum LpError {
    Infeasible,
}
