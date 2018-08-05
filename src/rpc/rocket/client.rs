use super::*;
use rpc::CallOutTrait;

pub struct ConvertImpl;

impl CallOutTrait for ConvertImpl {
    fn convert(carrier: &Carrier<ConverterInfo>) -> Result<ConverterReturned> {
        let _para = CallOutParameter::new(carrier);

        unimplemented!()
    }
}
