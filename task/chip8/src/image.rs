use crate::data::Address;

use thiserror_no_std::Error;

////////////////////////////////////////////////////////////////////////////////

pub trait Image {
    fn load_into_memory(&self, memory: &mut [u8; Address::DOMAIN_SIZE]);
    fn entry_point(&self) -> Address;
}

////////////////////////////////////////////////////////////////////////////////

#[derive(Error, Debug)]
pub enum Ch8ImageError {
    #[error("image is too big")]
    TooBig,
}

#[derive(Copy, Clone)]
pub struct Ch8Image<T: AsRef<[u8]>> {
    data: T,
}

impl<T: AsRef<[u8]>> Ch8Image<T> {
    const BASE_ADDRESS: Address = Address::new(0x200);

    pub fn new(data: T) -> Result<Self, Ch8ImageError> {
        if data.as_ref().len() > Address::DOMAIN_SIZE {
            return Err(Ch8ImageError::TooBig);
        }

        Ok(Ch8Image { data })
    }
}

impl<T: AsRef<[u8]>> Image for Ch8Image<T> {
    fn load_into_memory(&self, memory: &mut [u8; Address::DOMAIN_SIZE]) {
        let data = self.data.as_ref();
        let entry_point = self.entry_point().as_usize();

        memory[entry_point..entry_point + data.len()].copy_from_slice(data);
    }

    fn entry_point(&self) -> Address {
        Self::BASE_ADDRESS
    }
}
