

use crate::messages::Message;
use crate::messages::MessageId;

use serde::{Serialize, Deserialize};


#[derive(Serialize, Deserialize, Debug, Default)]
#[repr(C)]
pub struct AccMsg {
    pub id: MessageId, 
    pub counter: u16,
    pub acc_x: i16,
    pub acc_y: i16,
    pub acc_z: i16, 
}


impl AccMsg
{
    pub fn new() -> Self {
        AccMsg{
            id: MessageId::AccMsgId, 
            counter: 0, 
            acc_x: 0, 
            acc_y: 0, 
            acc_z: 0
        }
    }
}

impl Message for AccMsg 
{
    #[cfg(feature = "std")]
    fn print_fields(&self) 
    {
        // The underlying type of id is a u16.
        println!("id: {:04X} counter: {:04X} Ax: {:.32} Ay: {:.32} Az: {:.32}", self.id as u16, self.counter, self.acc_x, self.acc_y, self.acc_z)
    }
}

