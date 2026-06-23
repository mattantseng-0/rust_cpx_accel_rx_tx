use serde_repr::{Serialize_repr, Deserialize_repr};

#[repr(u16)]
#[derive(Serialize_repr, Deserialize_repr, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MessageId {
    #[default]
    UnknownId   = 0x0000, 
    AccMsgId    = 0x0001,
}