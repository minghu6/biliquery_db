use std::{
    cmp::Reverse,
    collections::BinaryHeap,
    fs::File,
    io::{BufReader, BufWriter, Read, Write, Seek},
    mem::size_of,
    path::PathBuf,
};

use crc32fast::hash as crc32;
use indicatif::{ProgressBar, ProgressStyle};
use m6coll::{array, Array, BitMap, Entry, ToLeBytes};

use crate::data::{TblMeta, UInt};
use crate::query::load_dbmeta;
use crate::{query::load_dup_dbmeta, shell::*};


// < 512 * 1024 * 1024 = 5_3687_0912
const BUNDLE_LEN: u32 = 1_0240_0000; // 1亿 (100_000 * 1024)
const BILI2_KEY_SIZE: u32 = 4;
const BILI2_VAL_SIZE: u32 = 4;
const BILI2_HASHUID_TBL_META: TblMeta = TblMeta {
    len: BUNDLE_LEN as u64,
    keysize: BILI2_KEY_SIZE,
    valsize: BILI2_VAL_SIZE,
};


////////////////////////////////////////////////////////////////////////////////
//// Reader && Writer


///////////////////////////////////////
//// Reader

pub(crate) struct TblReader {
    meta: TblMeta,
    cnt: u64,
    buf: Array<u8>,
    reader: BufReader<File>,
}

impl TblReader {
    pub(crate) fn new(meta: TblMeta, ty: TblTy) -> Self {
        let buf = array![0; (meta.keysize + meta.valsize) as usize];

        let file = if let Ok(res) = File::open(ty.pathbuf()) {
            res
        } else {
            panic!("Unable to open file {:#?}", &ty.pathbuf())
        };

        let mut reader = BufReader::new(file);
        // let mut reader = file;

        // skip meta
        let mut tmp_buf = [0; size_of::<TblMeta>()];
        reader.read(&mut tmp_buf).unwrap();

        Self {
            meta,
            cnt: 0,
            buf,
            reader,
        }
    }

    pub(crate) fn read_item(&mut self) -> Result<Entry<UInt, UInt>, ()> {
        if self.cnt == self.meta.len {
            return Err(());
        }

        self.reader.read(&mut self.buf[..]).unwrap(); // should be same with

        let key = UInt::from_slice(&self.buf[..self.meta.keysize as usize]);
        let val = UInt::from_slice(&self.buf[self.meta.keysize as usize..]);

        self.cnt += 1;

        Ok(Entry(key, val))
    }

    pub(crate) fn into_reader(self) -> impl Read + Seek {
        self.reader
    }
}


///////////////////////////////////////
//// Writer

struct DBWriter {
    id_cnt: TblTy,
    tblmeta: TblMeta,
}

impl DBWriter {
    fn init(tblmeta: TblMeta, ty: TblTy) -> Self {
        Self {
            id_cnt: ty,
            tblmeta,
        }
    }

    fn nxt_tbl_writer(&mut self) -> TblWriter {
        let tblwriter = TblWriter::new(self.tblmeta, self.id_cnt);
        self.id_cnt = self.id_cnt.add();
        tblwriter
    }
}

struct TblWriter {
    meta: TblMeta,
    ty: TblTy,
    cnt: u64,
    writer: BufWriter<File>,
}

impl TblWriter {
    fn new(meta: TblMeta, ty: TblTy) -> Self {
        ty.init();

        let ty = ty;
        let file = File::create(ty.pathbuf()).unwrap();
        let mut writer = BufWriter::new(file);

        // skip meta
        writer.write_all(&meta.to_le_bytes()).unwrap();

        TblWriter {
            meta,
            ty,
            cnt: 0,
            writer,
        }
    }

    #[inline]
    fn path(&self) -> PathBuf {
        self.ty.pathbuf()
    }

    #[inline]
    fn is_end(&self) -> bool {
        self.cnt == self.meta.len
    }

    fn write_item(&mut self, buf: &[u8]) -> Result<(), ()> {
        if self.is_end() {
            return Err(());
        }

        debug_assert_eq!(
            (self.meta.keysize + self.meta.valsize) as usize,
            buf.len()
        );

        self.writer.write_all(buf).unwrap(); // should be same with

        self.cnt += 1;

        Ok(())
    }
}


////////////////////////////////////////////////////////////////////////////////
//// Collision Resolver

#[derive(Clone, Copy)]
pub(crate) enum CollisionResolver {
    Rehash,  // Hash(Hash(x)),
}

impl CollisionResolver {
    fn resolve(&self, hashval: u32, _x: u32) -> u32 {
        match self {
            CollisionResolver::Rehash => crc32(hashval.to_string().as_bytes()),
        }
    }
}



////////////////////////////////////////////////////////////////////////////////
//// Service


pub fn gen_data_bili2(id: u32) {
    let mut writer = TblWriter::new(BILI2_HASHUID_TBL_META, TblTy::Dup(id));

    debug_assert!(BUNDLE_LEN % 1024 == 0);
    let mut heap = BinaryHeap::with_capacity(BUNDLE_LEN.try_into().unwrap());

    let unit = 1000;
    let start = id * BUNDLE_LEN + 1;
    let end = (id + 1) * BUNDLE_LEN + 1;

    let pb = ProgressBar::new((BUNDLE_LEN / unit).into());
    pb.set_style(
        ProgressStyle::default_spinner()
            .template("       calc crc32 {spinner:.green} [{elapsed_precise}] {pos:5}k/{len}k")

    );
    for i in start..end {
        let k = crc32(i.to_string().as_bytes());
        let v = i;

        heap.push(Reverse(Entry(k, v)));

        if (i - start) % unit == 0 {
            pb.set_position(((i - start) / unit).into());
        }
    }
    pb.finish();

    let pb = ProgressBar::new((BUNDLE_LEN / unit).into());
    pb.set_style(
        ProgressStyle::default_spinner()
            .template("heap pop && write {spinner:.green} [{elapsed_precise}] {pos:5}k/{len}k")

    );
    for i in 0..BUNDLE_LEN {
        let entry = heap.pop().unwrap().0;

        writer.write_item(&entry.to_le_bytes()).unwrap();

        if i % unit == 0 {
            pb.set_position((i / unit).into());
        }
    }
    pb.finish_with_message("Done.");
}


/// ReGeneration
pub fn gen_collision_data_bili2() {
    let dbmeta = load_dbmeta();

    let mut dup_db_writer =
        DBWriter::init(BILI2_HASHUID_TBL_META, TblTy::Dup(0));
    let mut dup_tbl_writer = dup_db_writer.nxt_tbl_writer();

    let pb = ProgressBar::new(0);
    pb.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} [{elapsed_precise}] {pos:>7}"),
    );

    let mut map = BitMap::new(u32::MAX as u128 + 1);
    let mut dup_cnt = 0;

    for (ty, meta) in dbmeta.0.iter() {

        let mut reader = TblReader::new(*meta, *ty);

        while let Ok(Entry(key, _val)) = reader.read_item() {
            let key_as_usize = key.into_u32() as usize;

            if map.test(key_as_usize) {
                if dup_tbl_writer.is_end() {
                    pb.println(format!(
                        "wrote into {}",
                        path2str(&dup_tbl_writer.path())
                    ));
                    dup_tbl_writer = dup_db_writer.nxt_tbl_writer();
                }
                dup_tbl_writer.write_item(&reader.buf[..]).unwrap();
                dup_cnt += 1;

                if dup_cnt % 1000 == 0 {
                    pb.set_position(dup_cnt);
                }
            } else {
                map.set(key_as_usize)
            }
        }

        pb.println(format!("wrote into {}", path2str(&dup_tbl_writer.path())));
    }

    pb.set_length(dup_cnt);
    pb.finish();
}


/// Collision Resolve
pub fn gen_collision_resolve_data_bili2() {
    let dbmeta = load_dup_dbmeta();
    let resolve = CollisionResolver::Rehash;
    let mut resolve_db_writer = DBWriter::init(
        BILI2_HASHUID_TBL_META,
        TblTy::Resolve(0, resolve),
    );
    let mut resolv_tbl_writer = resolve_db_writer.nxt_tbl_writer();

    let pb = ProgressBar::new(0);
    pb.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} [{elapsed_precise}] {pos:>7}"),
    );

    let mut dup_cnt = 0;

    for (ty, meta) in dbmeta.0.iter() {
        let mut reader = TblReader::new(*meta, *ty);

        while let Ok(Entry(key, val)) = reader.read_item() {
            let key  = key.into_u32();
            let val = val.into_u32();
            let new_key = resolve.resolve(key, val);
            let entry = Entry(new_key, val);

            if resolv_tbl_writer.is_end() {
                pb.println(format!(
                    "wrote into {}",
                    path2str(&resolv_tbl_writer.path())
                ));
                drop(resolv_tbl_writer);
                resolv_tbl_writer = resolve_db_writer.nxt_tbl_writer();
            }
            resolv_tbl_writer.write_item(&entry.to_le_bytes()).unwrap();
            dup_cnt += 1;

            if dup_cnt % 1000 == 0 {
                pb.set_position(dup_cnt);
            }
        }

        drop(reader);
    }

    pb.println(format!("wrote into {}", path2str(&resolv_tbl_writer.path())));
    pb.set_length(dup_cnt);
    pb.finish();
}


#[cfg(test)]
mod tests {
    use super::gen_data_bili2;

    #[test]
    fn test_container() {
        /* TEST BINARY HEAP */
        use std::cmp::Reverse;
        use std::collections::BinaryHeap;

        let mut heap = BinaryHeap::new();

        heap.push(Reverse(2));
        heap.push(Reverse(2));
        heap.push(Reverse(2));

        assert_eq!(heap.pop(), Some(Reverse(2)));
        assert_eq!(heap.pop(), Some(Reverse(2)));
        assert_eq!(heap.pop(), Some(Reverse(2)));
        assert_eq!(heap.pop(), None);

        /* TEST BITVEC */
        use bitvec::bitvec;

        let mut barr = bitvec![0; 64];

        assert_eq!(barr.len(), 64);
        *barr.get_mut(0).unwrap() = true;
    }

    #[test]
    fn test_run_bili2() {
        gen_data_bili2(0);
    }

    #[test]
    fn test_hash_bili2() {
        use crc32fast::hash;

        let uid = 6487381u32;
        let uid_s = format!("{}", uid);
        println!("{}: -h-> {:0x}", uid, hash(uid_s.as_bytes()))
    }
}
