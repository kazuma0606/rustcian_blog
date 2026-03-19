//domain/service/id_generator.rs
// idGenerator
// 2025/7/8

pub trait IdGeneratorInterface {
    fn generate(&self) -> String;
}

pub struct UuidGenerator;

impl IdGeneratorInterface for UuidGenerator {
    fn generate(&self) -> String {
        uuid::Uuid::new_v4().to_string()
    }
}
