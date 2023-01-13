use flatty::{flat, portable::le::*, FlatVec};

#[flat(portable = true, sized = false, enum_type = "u8")]
pub enum InMsg {
    Ai(F64),
    Aai(FlatVec<I32, U16>),
    Waveform(FlatVec<I32, U16>),
    Bi(U16),
    MbbiDirect(U32),
    Stringin(FlatVec<u8, U16>),
}

#[flat(portable = true, sized = false, enum_type = "u8")]
pub enum OutMsg {
    Ao(F64),
    Aao(FlatVec<I32, U16>),
    Bo(U16),
    MbboDirect(U32),
    Stringout(FlatVec<u8, U16>),
}
