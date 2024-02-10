#![allow(dead_code)]

use std::error::Error;
use rand::Rng;

const RANDOM_CHAR_POOL: &[u8] = b"0123456789\
                                abcdefghijklmnopqrstuvwxyz\
                                ABCDEFGHIJKLMNOPQRSTUVWXYZ\
                                <>?,./;:[]{}~!@#$%^&*()_+-=";

fn _gen_string(size: usize, pool: &Vec<u8>) -> String {
    let mut rng = rand::thread_rng();

    (0..size)
        .map(|_| {
            let idx = rng.gen_range(0..pool.len());
            pool[idx] as char
        })
        .collect()
}

pub fn gen_password(size: usize) -> String {
    _gen_string(size, &RANDOM_CHAR_POOL[..].to_vec())
}

pub fn gen_uuid() -> String {
    // todo: 需要依据设备创建
    let uuid_size: Vec<u8> = vec![16, 4, 4, 4, 12];
    let pool = RANDOM_CHAR_POOL[..15].to_vec();

    let uuid_slices: Vec<String> = uuid_size.iter().map(|size| {
        _gen_string(*size as usize, &pool)
    }).collect();

    uuid_slices.join("-")
}

pub type CommonErr = Box<dyn Error>;
pub type CommomResult<T> = Result<T, CommonErr>;

pub fn common_err(err_message: &str, err_kind: Option<std::io::ErrorKind>) -> CommonErr {
    let err_kind = match err_kind {
        Some(kind) => kind,
        None => std::io::ErrorKind::Other,
    };
    Box::new(std::io::Error::new(err_kind, err_message))
}

pub mod config;
pub mod utils;

#[cfg(test)]
mod common_random_str {
    use crate::common::gen_uuid;

    use super::gen_password;

    #[test]
    fn test_gen_password() {
        let cases: Vec<(usize, usize)> = vec![
            (0, 0),
            (1, 1),
        ];
        let _ = cases.iter().map(|(size, expect)| {
            assert_eq!(gen_password(*size).len(), *expect);
        });
    }

    #[test]
    fn test_gen_uuid() {
        let uuid = gen_uuid();
        let slice: Vec<&str> = uuid.split("-").collect();
        assert_eq!(slice.len(), 5);
        let uuid_size: Vec<u8> = vec![16, 4, 4, 4, 12];
        for (idx, str) in slice.iter().enumerate() {
            assert_eq!(uuid_size[idx] as usize, (*str).len());
        }
        
    }
}
