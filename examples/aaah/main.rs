#![allow(unused_imports)]
#![allow(unused_macros)]
use std::{
    cmp::{max, min},
    collections::{BTreeMap, BTreeSet, HashMap, HashSet, VecDeque},
    io::{self, stdin, stdout, BufRead, Read, Write},
};

fn main() {
    let stdin = stdin();
    let stdin = stdin.lock();
    let mut lines = stdin.lines();

    let a = lines.next().unwrap().unwrap().chars().count();
    let b = lines.next().unwrap().unwrap().chars().count();

    if a < b {
        println!("no");
    } else {
        println!("go");
    }
}
