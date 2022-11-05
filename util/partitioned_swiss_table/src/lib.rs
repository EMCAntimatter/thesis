#![feature(allocator_api)]
#![allow(incomplete_features)]
#![feature(adt_const_params)]
#![allow(incomplete_features)]
#![feature(generic_const_exprs)]

pub mod table;

pub fn add(left: usize, right: usize) -> usize {
    left + right
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}
