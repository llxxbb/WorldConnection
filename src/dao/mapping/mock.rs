use super::*;

pub struct MappingDaoService;

impl MappingDao for MappingDaoService {
    fn get_relations(_from: &Thing) -> Result<Vec<Mapping>> {
        unimplemented!()
    }
}