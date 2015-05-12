use std::str::{FromStr, Split};
use std::iter::Filter;
use std::borrow::Borrow;


pub trait NextValue {
    fn next_value<T:FromStr>(&mut self) -> Result<T, ()>;
    fn nth_value<T:FromStr>(&mut self, i: usize) -> Result<T, ()>;
}

impl<I, T: Borrow<str>> NextValue for I
    where I: Iterator<Item=T>
{

    fn next_value<A:FromStr>(&mut self) -> Result<A, ()> {
        self.next().ok_or(())
        .and_then(|x| FromStr::from_str(x.borrow()).map_err(|_| ()))
    }

    fn nth_value<A:FromStr>(&mut self, i: usize) -> Result<A, ()> {
        self.nth(i).ok_or(())
        .and_then(|x| FromStr::from_str(x.borrow()).map_err(|_| ()))
    }

}

pub trait NextStr<'a> {
    fn next_str(&mut self) -> Result<&'a str, ()>;
    fn nth_str(&mut self, i: usize) -> Result<&'a str, ()>;
}

impl<'a, I> NextStr<'a> for I
    where I: Iterator<Item=&'a str>
{
    fn next_str(&mut self) -> Result<&'a str, ()> {
        return self.next().ok_or(());
    }
    fn nth_str(&mut self, i: usize) -> Result<&'a str, ()> {
        return self.nth(i).ok_or(());
    }
}

pub struct Words<'a> {
    src: &'a str,
    offset: usize,
}

impl<'a> Iterator for Words<'a> {
    type Item = &'a str;
    fn next(&mut self) -> Option<&'a str> {
        unimplemented!();
    }
}

pub fn words<'a, 'b: 'a, B: Borrow<str> + ?Sized + 'a>(src: &'b B) -> Words<'a> {
    return Words { src: src.borrow(), offset: 0 };
}
