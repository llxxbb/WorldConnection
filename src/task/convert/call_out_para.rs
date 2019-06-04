use nature_common::{CallOutParameter, ConverterReturned, NatureError, Protocol, Result};
use nature_db::Mission;

use crate::task::{TaskForConvert, ExecutorTrait, HttpExecutorImpl, LocalExecutorImpl};

static HTTP_CALLER: &dyn ExecutorTrait = &HttpExecutorImpl;
static LOCAL_RUST_CALLER: &dyn ExecutorTrait = &LocalExecutorImpl;


pub struct CallOutParaWrapper;

impl CallOutParaWrapper {
    pub fn gen_and_call_out(task: &TaskForConvert, carrier_id: Vec<u8>, mission: &Mission) -> Result<ConverterReturned> {
        let para = CallOutParameter {
            from: task.from.clone(),
            last_status: task.last_status.clone(),
            carrier_id,
        };
        let executer = Self::get_executer(&mission.executor.protocol)?;
        Ok(executer.execute(&mission.executor.url, &para))
    }

    fn get_executer(protocol: &Protocol) -> Result<&'static dyn ExecutorTrait> {
        match protocol {
            Protocol::Http => Ok(HTTP_CALLER),
            Protocol::LocalRust => Ok(LOCAL_RUST_CALLER),
            _ => Err(NatureError::ConverterProtocalError(format!("Did not implement for protocal : {:?}", protocol)))
        }
    }
}
