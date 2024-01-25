
#[derive(Debug)]
pub struct DeviceConfigError {
   pub message: String
}


impl DeviceConfigError {
    pub fn fro_str(s: &str) -> Self {
        Self{
            message: s.to_string()
        }
    }

    pub fn new(s: String) -> Self {
        Self{
            message: s
        }
    }
}