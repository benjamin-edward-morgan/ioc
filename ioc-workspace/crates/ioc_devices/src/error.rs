use ioc_core::error::IocBuildError;

#[derive(Debug)]
pub struct DeviceConfigError {
   pub message: String
}


impl DeviceConfigError {
    pub fn from_str(s: &str) -> Self {
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


impl From<DeviceConfigError> for IocBuildError {
    fn from(err: DeviceConfigError) -> Self {
        IocBuildError::from_string(err.message)
    }
}