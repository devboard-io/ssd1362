#![no_std]

extern crate embedded_hal;

pub mod interface;
mod command;
pub mod error;
pub mod display;


#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
