use arraydeque::{ArrayDeque};
use core::fmt;
use core::cmp::max;

use generic_array::{ArrayLength, GenericArray, sequence::GenericSequence};
#[derive(Debug)]
struct Iter<'a, N: ArrayLength<u8>> {
    buffer: &'a Ringbuffer<N>,
    index: i32,
    step: i32,
}

impl<'a, N> Iterator for Iter<'a, N>
    where N: ArrayLength<u8>
{

    type Item = &'a[u8];

    fn next(&mut self) -> Option<&'a [u8]> {

        let i = self.index;
        self.index = self.index + self.step;
        if i < 0 {
            None
        } else {
            self.buffer.element(i as usize)
        }
    }
}

#[derive(Eq, PartialEq, Debug)]
pub enum Error {
    NoSpace,
}

pub struct Ringbuffer<N: ArrayLength<u8>> {
    buffer: GenericArray<u8, N>,
    line_pointers: ArrayDeque<[usize; 16]>,
    wp: usize,
    skipped: usize,
}


impl<N: ArrayLength<u8>>fmt::Debug for Ringbuffer<N> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Ringbuffer")
        .field("wp", &self.wp)
        .field("skipped", &self.skipped)
        .field("free", &self.free())
        .field("line_pointers", &self.line_pointers)
        .field("buffer", &self.buffer)
         .finish()
    }
}

impl<N: ArrayLength<u8>> Ringbuffer<N> {

    pub fn new() -> Self {
        Self {
            buffer: GenericArray::generate(|_| 0u8),
            line_pointers: ArrayDeque::new(),
            wp: 0,
            skipped: 0,
        }
    }

    /// try to add slice of bytes as a new element
    /// throws error if there is no space in the buffer
    pub fn try_add(&mut self, bytes: &[u8]) -> Result<(), Error> {
        if bytes.len() > self.free() {
            return Err(Error::NoSpace);
        }

        self.wrap_wp(bytes.len());
        match self.line_pointers.push_back(self.wp) {
            Ok(_) => {
                // #[cfg(test)]
                // println!("wp: {}, skipped: {}", self.wp, self.skipped);
                for b in bytes {
                    self.buffer[self.wp] = *b;
                    self.wp += 1;
                    self.wp %= self.buffer.len();
                }
            },
            Err(_err) => {
                // buffer full..
                return Err(Error::NoSpace);
            }
        };

        Ok(())
    }

    pub fn add(&mut self, bytes: &[u8]) {
        while let Err(_err) = self.try_add(bytes) {
            #[cfg(test)]
            println!("No space, will pop 1 element");
            if !self.pop() {
                // nothing to pop anymore
                break;
            }
        }
    }

    pub fn iter(&self, offset: usize) -> impl Iterator<Item=&[u8]> {

        Iter {
            buffer: &self,
            index: offset as i32,
            step: 1,
        }
    }

    pub fn reverse_iter(&self, offset: usize) -> impl Iterator<Item=&[u8]> {

        let i = self.line_pointers.len() as i32 - offset as i32 - 1;

        Iter {
            buffer: &self,
            index: i,
            step: -1,
        }
    }

    pub fn element(&self, index: usize) -> Option<&[u8]> {
        if self.line_pointers.len() == 0 || index > self.line_pointers.len() {
            return None;
        }

        let start = self.line_pointers[index];

        // #[cfg(test)]
        // println!("len {}, index+1 {}", self.line_pointers.len(), index+1);
        #[cfg(test)]
        println!("wp {}, start {}", self.wp, start);


        let end = if self.wp < start {
            self.buffer.len() - self.skipped // todo: when or how to reset?
        } else if self.line_pointers.len() <= index+1 {
            self.wp
        } else {
            self.line_pointers[index+1]
        };

        Some(&self.buffer[start..end])
    }

    /// Pop one element from the front of the buffer.
    /// That is, the element that was added first.
    /// Returns bool indicating whether anything was popped or not.
    pub fn pop(&mut self) -> bool {
        let popped = match self.line_pointers.pop_front() {
            Some(i) => {

                let j = match self.line_pointers.front() {
                    Some(el ) => *el,
                    None => i
                };

                // reset skipped bytes when next pointer is smaller than popped pointer
                // Because it means we have reached the end of the buffer. So any skipped bytets will merge
                if j < i {
                    #[cfg(test)]
                    println!("Reset skipped bytes");
                    self.skipped = 0;
                }

                true
            },
            None => false
        };

        if self.line_pointers.len() == 0 {
            self.wp = 0
        }

       popped
    }

    pub fn used(&self) -> usize {
        self.buffer.len() - self.free()
    }

    /// Returns optimal write pointer for given element size
    /// It makes sure that elements are always in continuous memory
    pub fn wrap_wp(&mut self, size: usize) {

        let first_index: usize = match self.line_pointers.front() {
            Some(&i) => i,
            None => 0
        };

        self.wp = if self.wp >= first_index {
            if self.buffer.len() - self.wp - 1 >= size {
                self.wp
            } else {
                // wrap write pointer to start (=0) even though there
                // are some bytes still free at the end of the buffer.
                // keep track of those skipped bytes
                self.skipped = self.buffer.len() - self.wp;
                0
            }
        } else {
           self.wp
        };
    }

    /// Get number of free elements in ringbuffer
    pub fn free(&self) -> usize {
        let first_index: usize = match self.line_pointers.front() {
            Some(&i) => i,
            None => 0
        };

        // if you think very deeply about this:
        // there is a specific case to win a single byte.
        // But I don't think it is worth the complexity..
        let free = if self.wp >= first_index {
            max(self.buffer.len() - self.wp, first_index)
        } else {
            first_index - self.wp
        };

        // #[cfg(test)]
        // println!("wp {}, first {}, free {}", self.wp, first_index, free);

        // -1 to prevent wp == first_index
        free - 1
    }
}

#[cfg(test)]
#[macro_use]
extern crate std;
#[cfg(test)]
mod tests {
    use super::*;
    use generic_array::typenum::{U1, U2, U8, U32, U40};

    #[test]
    fn it_handles_full_buffer() {
        let buffer: Ringbuffer::<U1> = Ringbuffer::new();
        assert_eq!(buffer.free(), 0);

        let mut buffer: Ringbuffer::<U2> = Ringbuffer::new();
        buffer.try_add(&[1]).ok();
        assert_eq!(buffer.free(), 0);
    }

    #[test]
    fn it_knows_its_length() {

        let mut buffer: Ringbuffer::<U8> = Ringbuffer::new();
        assert_eq!(buffer.free(), 7);

        buffer.try_add(&[1,2,3,4]).ok();
        assert_eq!(buffer.free(), 3);
        // [1,2,3,4,_,_,_,x]

        buffer.try_add(&[5, 6]).ok();
        assert_eq!(buffer.free(), 1);
        // [1,2,3,4,5,6,_,x]
        println!("{:?}", buffer);

        //make space by removing first batch (=4)
        buffer.pop();
        // [_,_,_,x,5,6,_,x]
        assert_eq!(buffer.free(), 3);

        // println!("{:?}", buffer);
        buffer.try_add(&[7, 8, 9]).ok();
        // [7,8,9,x,5,6,_,x]
        assert_eq!(buffer.free(), 0);


        let should_be_error = buffer.try_add(&[7, 8]).unwrap_err();
        assert_eq!(should_be_error, Error::NoSpace);
    }

    #[test]
    fn it_pops_elements_correctly() {
        let mut buffer: Ringbuffer::<U8> = Ringbuffer::new();
        assert_eq!(buffer.free(), 7);

        buffer.try_add(&[1,2]).ok();
        buffer.try_add(&[3,4,5,6]).ok();
        assert_eq!(buffer.free(), 1);
        buffer.pop();
        assert_eq!(buffer.free(), 1);
        buffer.pop();
        assert_eq!(buffer.free(), 7);

    }

    #[test]
    fn it_returns_correct_slice() {
        let mut buffer: Ringbuffer::<U8> = Ringbuffer::new();
        buffer.try_add(&[1,2]).ok();
        buffer.try_add(&[3,4,3]).ok();
        buffer.try_add(&[5,6]).ok();

        let mut iterator = buffer.reverse_iter(0);

        let slice = iterator.next().unwrap();

        assert_eq!(slice.len(), 2);
        assert_eq!(slice[0], 5);
        assert_eq!(slice[1], 6);

        let slice = iterator.next().unwrap();

        assert_eq!(slice.len(), 3);
        assert_eq!(slice[0], 3);
        assert_eq!(slice[1], 4);

        let slice = iterator.next().unwrap();

        assert_eq!(slice.len(), 2);
        assert_eq!(slice[0], 1);
        assert_eq!(slice[1], 2);

        let slice = iterator.next();

        assert_eq!(slice, None);

    }

    fn drop<'a>(_d: impl Iterator<Item=&'a[u8]>) {
        // bye iterator
        println!("drop iterator")
    }

    #[test]
    fn it_iterator_and_pop() {
        let mut buffer: Ringbuffer::<U8> = Ringbuffer::new();
        buffer.try_add(&[1,2]).ok();
        buffer.try_add(&[3,4,3]).ok();
        buffer.try_add(&[5,6]).ok();

        let iterator = buffer.reverse_iter(0);
        drop(iterator);
        buffer.pop();
    }

    #[test]
    fn it_errors_on_overflow() {

        // first test overflow error for buffer
        let mut buffer: Ringbuffer::<U8> = Ringbuffer::new();
        assert_eq!(buffer.free(), 7);

        buffer.try_add(&[1,2]).ok();
        buffer.try_add(&[3,4,5,6]).ok();
        let should_be_error = buffer.try_add(&[7, 8]).unwrap_err();

        assert_eq!(should_be_error, Error::NoSpace);


        // second test overflow error for index array. It has capacity of 16.
        let mut buffer: Ringbuffer::<U32> = Ringbuffer::new();

        for i in 0..16 {
            buffer.try_add(&[i]).unwrap();
        }

        let should_be_error = buffer.try_add(&[0xEE]).unwrap_err();
        assert_eq!(should_be_error, Error::NoSpace);

    }

    #[test]
    fn it_correctly_wraps_write_pointer() {

        // first test overflow error for buffer
        let mut buffer: Ringbuffer::<U8> = Ringbuffer::new();

        // wp = 0
        buffer.add(&[1,2,3,4]);
        // wp = 4
        buffer.add(&[5,6]);
        // wp = 6
        let f = buffer.free();
        assert_eq!(f, 1);

        buffer.pop();
        let f = buffer.free();
        assert_eq!(f, 3);

        // wp = 6 for size < 2, wp = 0 size >= 2
        buffer.add(&[7,8]);
        let f = buffer.free();

        assert_eq!(buffer.free(), 1);
    }

    #[test]
    fn it_wrapps() {

        // first test overflow error for buffer
        let mut buffer: Ringbuffer::<U8> = Ringbuffer::new();

        // [1,2,_,_,_,_,_,x]
        buffer.add(&[1,2]);
        // [1,2, 3,4,3, _,_,x]
        buffer.add(&[3,4,3]);
        // [1,2, 3,4,3, 5,6,x]
        buffer.add(&[5,6]);

        let mut iterator = buffer.reverse_iter(0);

        let slice = iterator.next().unwrap();

        assert_eq!(slice.len(), 2);
        assert_eq!(slice[0], 5);
        assert_eq!(slice[1], 6);

        drop(iterator);

        // [7,8, _,_,x, 5,6,x]
        buffer.add(&[7,8]);

        println!("{:?}", buffer);
        let mut iterator = buffer.iter(0);

        let slice = iterator.next().unwrap();

        assert_eq!(slice.len(), 2);
        assert_eq!(slice[0], 5);
        assert_eq!(slice[1], 6);

        let slice = iterator.next().unwrap();

        assert_eq!(slice.len(), 2);
        assert_eq!(slice[0], 7);
        assert_eq!(slice[1], 8);
    }

    #[test]
    fn it_access_last_element_when_wp_is_wraped() {
         // first test overflow error for buffer
         let mut buffer: Ringbuffer::<U8> = Ringbuffer::new();

         // [1,2,_,_,_,_,_,x]
         buffer.add(&[1,2]);
         // [1,2, 3,4,3, _,_,x]
         buffer.add(&[3,4,3]);
         // [1,2, 3,4,3, 5,6,x]
         buffer.add(&[5,6]);

        //[_,_, 3,4,3, 5,6,x]
        buffer.pop();
        buffer.free();
        buffer.wrap_wp(2);
        buffer.free();

        let mut iterator = buffer.reverse_iter(0);
        let slice = iterator.next().unwrap();

        assert_eq!(slice.len(), 2);
        assert_eq!(slice[0], 5);
        assert_eq!(slice[1], 6);

    }


    #[test]
    fn it_resets_skipped_bytes() {
        // first test overflow error for buffer
        let mut buffer: Ringbuffer::<U8> = Ringbuffer::new();

        // [1,2,3,4,_,_,_,x]
        buffer.add(&[1,2]);
        buffer.add(&[3,4]);
        buffer.add(&[5,6]);
        buffer.add(&[7,8]);

        // [7, 8, 0, x, 5, 6, 0, 0]

        buffer.pop();
        println!("{:?}", buffer);

        buffer.add(&[1,2,3,4,5]);
        buffer.add(&[9,9]);
        println!("{:?}", buffer);

        let mut iterator = buffer.reverse_iter(0);
        let slice = iterator.next().unwrap();

    }

    #[test]
    fn it_handles_zero_length_gracefully() {
        // first test overflow error for buffer
        let buffer: Ringbuffer::<U8> = Ringbuffer::new();
        let mut iterator = buffer.reverse_iter(0);

        let slice = iterator.next();
        assert_eq!(slice, None);
        println!("{:?}", slice);
        drop(iterator);

        // offset while empty
        let mut iterator = buffer.reverse_iter(1);

        let slice = iterator.next();
        assert_eq!(slice, None);
        println!("{:?}", slice);
        drop(iterator);

        let mut iterator = buffer.iter(0);

        let slice = iterator.next();
        assert_eq!(slice, None);
        println!("{:?}", slice);
        drop(iterator);



    }

    #[test]
    fn it_handles_index_larger_than_len() {

        let mut buffer: Ringbuffer::<U8> = Ringbuffer::new();

        // offset more than length
        buffer.add(&[9,9,9,9,9]);
        let mut iterator = buffer.reverse_iter(2);

        let slice = iterator.next();
        assert_eq!(slice, None);
        // assert_eq!(slice.unwrap(), [9,9,9,9,9]);
        drop(iterator);

        let mut iterator = buffer.iter(2);

        let slice = iterator.next();
        assert_eq!(slice, None);
        // println!("{:?}", slice);
        // drop(iterator);

    }

    #[test]
    fn it_handles_real_world_example() {
        let mut buffer: Ringbuffer::<U40> = Ringbuffer::new();

        buffer.add(&[1,1,255]);
        buffer.add(&[2,2,255]);
        buffer.add(&[3,3, 255]);

        let a4 = [4u8; 30];
        buffer.add(&a4);

        assert_eq!(buffer.free(), 0);

    }

}
