use crate::instance::conversion::error::InstanceConversionError;
use crate::instance::Instance;

mod error;

struct PolyMcInstance{


}

impl TryInto<Instance> for PolyMcInstance{
    type Error = InstanceConversionError;

    fn try_into(self) -> Result<Instance, Self::Error> {

        todo!()
    }
}
