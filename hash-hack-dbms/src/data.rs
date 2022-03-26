//! Table Scheme
//!
//! TblMeta(8 + 4 + 4 = 16)
//! TblItem ...
//!
//!

use std::{mem::size_of, fmt::Display};
use serde_derive::Serialize;

use m6coll::{ ToLeBytes, Array, Entry };
use sorted_vec::SortedVec;

use crate::shell::{TblTy, path2str};

#[allow(unused)]
pub(crate) struct DBMeta(
    pub(crate) SortedVec<Entry<u32, TblMeta>>
);

pub(crate) struct DBMeta2(
    pub(crate) Vec<(TblTy, TblMeta)>
);


#[cfg(target_pointer_width="64")]
#[derive(Clone, Copy)]
#[repr(C)]
pub struct TblMeta {
    pub len: u64,
    pub keysize: u32,
    pub valsize: u32
}


pub struct Tbl {
    pub meta: TblMeta,
    pub data: *mut u8
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[repr(C)]
pub enum UInt {
    U32(u32),
    U64(u64)
}


impl TblMeta {

    /// From raw head
    pub unsafe fn from_raw(raw: *mut u8) -> Self {

        let mut p = raw;

        let len = *(p as *mut u64);
        p = p.add(size_of::<u64>());

        let keysize = *(p as *mut u32);
        p = p.add(size_of::<u32>());

        let valsize = *(p as *mut u32);

        Self {
            len,
            keysize,
            valsize,
        }
    }

    pub fn tbl_bytes(&self) -> u64 {
        let unit = (self.keysize + self.valsize) as u64;
        let data = unit * self.len;

        data + size_of::<Self>() as u64
    }

}

impl ToLeBytes for TblMeta {
    fn to_le_bytes(&self) -> Array<u8> {
        let cap = 4 + 4 + 8;
        let mut arr = Array::new(cap);

        arr[0..8].copy_from_slice(&self.len.to_le_bytes());
        arr[8..12].copy_from_slice(&self.keysize.to_le_bytes());
        arr[12..].copy_from_slice(&self.valsize.to_le_bytes());

        arr
    }
}

impl Tbl {
    /// From raw head
    pub unsafe fn from_raw(raw: *mut u8) -> Self {
        let meta = TblMeta::from_raw(raw);
        let data = raw.add(size_of::<TblMeta>());  // 16 bytes alignments maybe Itanium spec

        Self {
            meta,
            data,
        }
    }
}



impl DBMeta2 {
    pub fn total_items(&self) -> u64 {
        self.0.iter().fold(0,|acc, ( _, meta)| acc + meta.len )
    }
}


impl Display for DBMeta2 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {

        let total_tbls = self.0.len();
        let total_items = self.total_items();
        let total_bytes = self.0.iter().fold(0,|acc, ( _, meta) | acc + meta.tbl_bytes() );

        writeln!(f,
            "total {} tbls, {} items, {} bytes:",
            total_tbls, total_items, total_bytes
        )?;

        for (ty, meta) in self.0.iter() {
            writeln!(f, "{}: {} items, {} bytes", path2str(&ty.pathbuf()), meta.len, meta.tbl_bytes())?;
        }

        Ok(())
    }
}


impl UInt {
    pub fn len(&self) -> usize {
        match self {
            UInt::U32(_) => 4,
            UInt::U64(_) => 8,
        }
    }

    pub fn from_slice(slice: &[u8]) -> Self {
        match slice.len() {
            4 => {
                UInt::U32(u32::from_le_bytes(slice.try_into().unwrap()))
            },
            8 => {
                UInt::U64(u64::from_le_bytes(slice.try_into().unwrap()))
            }
            _ => unimplemented!()
        }
    }

    pub fn into_u32(self) -> u32 {
        match self {
            UInt::U32(v) => v,
            _ => unreachable!("{:#?}", self),
        }
    }
}





#[cfg(test)]
mod tests {
    use std::mem::size_of;

    use super::TblMeta;


    #[test]
    fn check_env() {
        assert_eq!(16, size_of::<TblMeta>())
    }


}