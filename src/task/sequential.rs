use std::collections::HashMap;
use std::collections::HashSet;

use chrono::prelude::*;
use serde_json;

use crate::system::*;

use super::*;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SerialFinished {
    pub succeeded_id: Vec<u128>,
    pub errors: Vec<String>,
}

impl SerialFinished {
    pub fn to_virtual_instance(&self, context_for_finish: &str) -> Result<Instance> {
        let json = serde_json::to_string(self)?;
        let mut context: HashMap<String, String> = HashMap::new();
        context.insert(context_for_finish.to_string(), json);
        let time = Local::now().timestamp();
        Ok(Instance {
            id: 0,
            data: InstanceNoID {
                thing: Thing::new_with_type(&SYS_KEY_SERIAL, ThingType::System)?,
                event_time: time,
                execute_time: time,
                create_time: time,
                content: String::new(),
                context,
                status: HashSet::new(),
                status_version: 0,
                from: None,
            },
        })
    }
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct TaskForSerialWrapper;

impl TaskForSerialWrapper {
    pub fn save<FC, FS>(serial: TaskForSerial, checker: &FC, saver: FS) -> Result<SerialFinished>
        where FC: Fn(&Thing) -> Result<RawThingDefine>,
              FS: Fn(&Instance) -> Result<usize>
    {
        let mut errors: Vec<String> = Vec::new();
        let mut succeeded_id: Vec<u128> = Vec::new();
        for mut instance in serial.instances {
            instance.change_thing_type(ThingType::Business);
            instance.data.thing = serial.thing.clone();
            if let Err(err) = instance.check_and_fix_id(checker) {
                errors.push(format!("{:?}", err));
                continue;
            }
            match saver(&instance) {
                Ok(_) => succeeded_id.push(instance.id),
                Err(err) => match err {
                    NatureError::DaoEnvironmentError(_) => return Err(err),
                    NatureError::DaoDuplicated(_) => succeeded_id.push(instance.id),
                    _ => {
                        errors.push(format!("{:?}", err));
                        continue;
                    }
                }
            }
        }
        Ok(SerialFinished { succeeded_id, errors })
    }
}
