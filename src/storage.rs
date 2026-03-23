use std::collections::HashMap;
use crate::page_parser::Tender;
use crate::error::Error;

const PATH: &str = "./data.bin";

pub struct TendersStorage(HashMap<usize, Tender>);
impl TendersStorage {
    pub fn init() -> Result<Self, Error> {
        fn reed_or_new() -> Result<HashMap<usize, Tender>, Error> {
            use postcard::from_bytes;
            match std::fs::read(PATH) {
                Ok(bytes) => {
                    match from_bytes(&bytes) {
                        Ok(map)=> Ok(map),
                        Err(e) => Err(Error::PostardError(e)),
                    }
                    
                },
                Err(_) => Ok(HashMap::new())
            }
        }
        reed_or_new().map(|map| Self(map))
    }
    pub fn save(&self) {
        use postcard::to_allocvec;
        if let Ok(bytes) = to_allocvec(&self.0) {
            let _ = std::fs::write(PATH, bytes);
        }
    }
    pub fn contains(&self, tender: &Tender) -> bool {
        self.0.contains_key(&tender.id)
    }
    pub fn push(&mut self, tender: Tender) {
        self.0.insert(tender.id, tender);
    }
}
impl Drop for TendersStorage {
    fn drop(&mut self) {
        self.save();
    }
}
