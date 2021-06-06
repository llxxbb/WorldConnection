use std::panic::catch_unwind;

use crate::db::RawTask;
use crate::db::flow_tool::state_check;
use crate::domain::*;
use crate::nature_lib::middleware::builtin_converter::BuiltIn;
use crate::nature_lib::middleware::filter::convert_before;
use crate::nature_lib::task::{http_execute_async, TaskForConvert};
use crate::nature_lib::task::local_common::local_execute;

pub type Execute = fn(para: &ConverterParameter) -> ConverterReturned;

pub async fn call_executor(task: &mut TaskForConvert, raw: &RawTask, last_target: &Option<Instance>, master: Option<Instance>) -> ConverterReturned {
    if let Some(ref last) = last_target {
        let demand = &task.target.down_selector;
        let _ = match demand {
            None => {}
            Some(demand) => {
                if !state_check(&last.states, &demand.last_none, &demand.last_all, &demand.last_any) {
                    return ConverterReturned::EnvError { msg: "target last instance unready".to_string() };
                }
            }
        };
    };
    &task.from;
    match convert_before(&mut task.from, task.target.convert_before.clone()).await {
        Err(NatureError::EnvironmentError(e)) => return ConverterReturned::EnvError { msg: e },
        Err(e) => return ConverterReturned::LogicalError { msg: e.to_string() },
        _ => ()
    };
    let para = ConverterParameter {
        from: task.from.clone(),
        last_state: last_target.clone(),
        task_id: raw.task_id.clone(),
        master,
        cfg: task.target.executor.settings.to_string(),
    };
    debug!("execute: from: {}, to : {}, executor: {}", task.from.meta, task.target.to.meta_string(), &task.target.executor.url);
    let rtn = match &task.target.executor.protocol {
        Protocol::Http => http_execute_async(&task.target.executor.url, &para).await,
        Protocol::LocalRust => match local_execute(&task.target.executor.url, &para).await {
            Ok(rtn) => rtn,
            Err(err) => ConverterReturned::EnvError { msg: err.to_string() }
        }
        Protocol::BuiltIn => match BuiltIn::get(&task.target.executor.url) {
            Ok(exe) => {
                match catch_unwind(|| { exe(&para) }) {
                    Ok(rtn) => {
                        rtn
                    }
                    Err(e) => {
                        warn!("{:?} return error: {:?}", task.target.executor.url, e);
                        ConverterReturned::LogicalError { msg: "executor implement error".to_string() }
                    }
                }
            }
            Err(_) => ConverterReturned::LogicalError { msg: "get built-in executor failed".to_string() }
        }
        _ => ConverterReturned::LogicalError { msg: format!("Did not implement for protocal : {:?}", &task.target.executor.protocol) },
    };
    rtn
}