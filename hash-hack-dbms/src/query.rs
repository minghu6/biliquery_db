use std::{
    cmp::{min, Ordering},
    fs::{read_dir, File},
    io::{self, Read, Seek, SeekFrom},
    mem::size_of,
};

use m6coll::Entry;
use regex::Regex;
use sorted_vec::SortedVec;

use crate::{
    data::{DBMeta2, TblMeta, UInt},
    gendata::TblReader,
};
use crate::{gendata::CollisionResolver, shell::*};


pub(crate) fn query_db(
    dbmeta: &DBMeta2,
    key: UInt,
) -> Result<Vec<UInt>, io::Error> {
    let mut res = vec![];

    for (ty, meta) in dbmeta.0.iter() {
        let mut reader = TblReader::new(*meta, *ty).into_reader();

        let unit_len = meta.keysize + meta.valsize;
        let meta_off = size_of::<TblMeta>() as u64;

        let mut key_cache = vec![0u8; meta.keysize as usize];
        let mut quick_cache = vec![0u8; 10 * 2 * unit_len as usize];

        // quick failed
        reader.seek(SeekFrom::End(0 - unit_len as i64))?;
        reader.read_exact(&mut key_cache[..])?;
        let local_key = UInt::from_slice(&key_cache[..]);
        if local_key < key {
            continue;
        }

        // SEEK PLAN
        let mut l = 0;
        let mut h = meta.len; // [l, h)

        while l < h {
            let pivot = (h + l) / 2;
            reader
                .seek(SeekFrom::Start(meta_off + pivot * unit_len as u64))?;

            reader.read_exact(&mut key_cache[..])?;
            let local_key = UInt::from_slice(&key_cache[..]);

            match key.cmp(&local_key) {
                Ordering::Less => {
                    h = pivot;
                }
                Ordering::Equal => {
                    // read +/- 10 slice

                    let half =
                        quick_cache.capacity() as u64 / unit_len as u64 / 2;

                    let el = pivot - min(pivot, half);
                    let eh = min(pivot + half, meta.len);
                    let cache_len = (eh - el) as usize;

                    reader.seek(SeekFrom::Start(
                        meta_off + el * unit_len as u64,
                    ))?;
                    reader.read_exact(
                        &mut quick_cache[..cache_len * unit_len as usize],
                    )?;

                    for i in 0..cache_len {
                        let base = i * unit_len as usize;

                        let local_key = UInt::from_slice(
                            &quick_cache[base..base + meta.keysize as usize],
                        );
                        let local_val = UInt::from_slice(
                            &quick_cache[base + meta.keysize as usize
                                ..base + unit_len as usize],
                        );

                        if local_key == key {
                            res.push(local_val);
                        }
                    }

                    break;
                }
                Ordering::Greater => {
                    l = pivot + 1;
                }
            }
        }
    }

    Ok(res)
}


pub fn query_bili2(id: u32) -> Result<Vec<u32>, io::Error> {
    let dbmeta = load_dbmeta();
    let raw_res = query_db(&dbmeta, UInt::U32(id))?;

    Ok(raw_res.into_iter().map(|uint| uint.into_u32()).collect())
}

pub fn query_collision_rehash_resolve(id: u32) -> Result<Vec<u32>, io::Error> {
    let resolve = CollisionResolver::Rehash;

    let dbmeta = resolve.load_dbmeta();

    let raw_res = query_db(&dbmeta, UInt::U32(id))?;

    Ok(raw_res.into_iter().map(|uint| uint.into_u32()).collect())
}


pub fn load_tblmeta(id: u32) -> TblMeta {
    let path = tbl_path(id);

    let mut file = File::open(path).unwrap();
    let mut buf = [0; size_of::<TblMeta>()];

    file.read_exact(&mut buf).unwrap();

    unsafe { TblMeta::from_raw(buf.as_mut_ptr()) }
}

pub(crate) fn load_dbmeta() -> DBMeta2 {
    let paths = read_dir("./").unwrap();
    let datareg = Regex::new("data([0-9]+)").unwrap();
    let mut coll = SortedVec::new();

    for path in paths {
        let dir_entry = path.unwrap();
        let name = path2str(&dir_entry.path());

        if let Some(cap) = datareg.captures(name.as_str()) {
            let id =
                u32::from_str_radix(cap.get(1).unwrap().as_str(), 10).unwrap();
            let meta = load_tblmeta(id);
            coll.insert(Entry(id, meta));
        }
    }

    let vec = coll
        .iter()
        .map(|Entry(id, meta)| (TblTy::Normal(*id), *meta))
        .collect();

    DBMeta2(vec)
}

pub(crate) fn load_dup_dbmeta() -> DBMeta2 {
    let paths = read_dir(tbl_dup_dir()).unwrap();
    let datareg = Regex::new("([0-9]+)").unwrap();
    let mut coll = SortedVec::new();

    for path in paths {
        let dir_entry = path.unwrap();
        let name = path2str(&dir_entry.path());

        if let Some(cap) = datareg.captures(name.as_str()) {
            let id =
                u32::from_str_radix(cap.get(1).unwrap().as_str(), 10).unwrap();
            let meta = load_tblmeta(id);
            coll.insert(Entry(id, meta));
        }
    }

    let vec = coll
        .iter()
        .map(|Entry(id, meta)| (TblTy::Dup(*id), *meta))
        .collect();

    DBMeta2(vec)
}

impl CollisionResolver {
    pub(crate) fn load_dbmeta(&self) -> DBMeta2 {
        match self {
            CollisionResolver::Rehash => {
                let paths = read_dir("data_cr_rehash").unwrap();
                let datareg = Regex::new("([0-9]+)").unwrap();
                let mut coll = SortedVec::new();

                for path in paths {
                    let dir_entry = path.unwrap();
                    let name = path2str(&dir_entry.path());

                    if let Some(cap) = datareg.captures(name.as_str()) {
                        let id = u32::from_str_radix(
                            cap.get(1).unwrap().as_str(),
                            10,
                        )
                        .unwrap();
                        let meta = load_tblmeta(id);
                        coll.insert(Entry(id, meta));
                    }
                }

                let vec = coll
                    .iter()
                    .map(|Entry(id, meta)| {
                        (TblTy::Resolve(*id, CollisionResolver::Rehash), *meta)
                    })
                    .collect();

                DBMeta2(vec)
            }
        }
    }
}



pub fn print_dbmeta() {
    let dbmeta = load_dbmeta();
    println!("{}", dbmeta);

    let db_dup_meta = load_dup_dbmeta();

    if !db_dup_meta.0.is_empty() {
        println!("Dup DB:");
        println!("{}", db_dup_meta)
    }
}
