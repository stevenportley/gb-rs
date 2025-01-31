use std::println;
use std::vec;
use std::vec::Vec;
use std::borrow::ToOwned;
use std::string::String;
use crate::cart::CartridgeData;

pub struct VecCart {
    rom: Vec<u8>,
    ram: Vec<u8>,
    save_path: Option<String>,
}

impl VecCart {
    pub fn from_slice(data: &[u8], save_dir: Option<&str>) -> Self {
        let header = crate::cart::get_cart_header(data);
        let rom = Vec::from(data);

        if let Some(dir) = save_dir {
            let file = dir.to_owned() + &header.title;
            let ram = std::fs::read(file.clone());

            let ram: Vec<u8> = if ram.is_ok() {
                ram.unwrap()
            } else {
                vec![0; header.ram_size as usize]
            };

            assert_eq!(ram.len(), header.ram_size as usize);

            Self {
                rom,
                ram,
                save_path: Some(file),
            }
        } else {
            let ram = vec![0; header.ram_size as usize];
            Self {
                rom,
                ram,
                save_path: None,
            }
        }
    }
}

impl Drop for VecCart {
    fn drop(&mut self) {
        self.save();
    }
}

impl CartridgeData for VecCart {
    type Rom = Vec<u8>;
    type Ram = Vec<u8>;

    fn rom(&self) -> &Self::Rom {
        &self.rom
    }

    fn rom_mut(&mut self) -> &mut Self::Rom {
        &mut self.rom
    }

    fn ram(&self) -> &Self::Ram {
        &self.ram
    }

    fn ram_mut(&mut self) -> &mut Self::Ram {
        &mut self.ram
    }

    fn save(&mut self) {
        if let Some(file) = &self.save_path {
            if std::fs::write(file, &self.ram).is_err() {
                println!("Unable to save the game!");
            }
        }

    }
}
