#[macro_use] extern crate nom;
#[macro_use] extern crate itertools;
extern crate crc;

mod parser;
mod writer;
mod internal;

#[derive(Debug)]
pub struct Nus3audioFile {
    pub files: Vec<AudioFile>
}

#[derive(Debug)]
pub struct AudioFile {
    pub id: u32,
    pub name: String,
    pub data: Vec<u8>
}

impl Nus3audioFile {
    pub fn new() -> Self {
        Nus3audioFile { files: vec![] }
    }
    
    pub fn from_bytes(data: &[u8]) -> Nus3audioFile {
        parser::take_file(
            &data[..]
        ).expect("Failed to parse file").1
    }

    pub fn open<P: AsRef<std::path::Path>>(path: P) -> Result<Nus3audioFile, std::io::Error> {
        Ok(Nus3audioFile::from_bytes(
            &std::fs::read(path)?[..]
        ))
    }
}

impl AudioFile {
    pub fn from_id(id: u32) -> Self {
        AudioFile { data: vec![], name: String::new(), id }
    }
    
    pub fn filename(&self) -> String {
        self.name.clone() + 
            if self.data.len() >= 4 {
                match &self.data[..4] {
                    b"OPUS" => ".lopus",
                    b"IDSP" => ".idsp",
                    _ => ".bin",
                }
            } else {
                ".bin"
            }
    }
}

#[cfg(test)]
mod tests {
    use super::*;


    static TEST_FILE: &[u8] = b"NUS3x\x00\x00\x00AUDIINDX\x04\x00\x00\x00\x01\x00\x00\x00TNID\x04\x00\x00\x00E\x00\x00\x00NMOF\x04\x00\x00\x00H\x00\x00\x00ADOF\x08\x00\x00\x00p\x00\x00\x00\x05\x00\x00\x00TNNM\n\x00\x00\x00test_name\x00JUNK\x0e\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00PACK\x10\x00\x00\x00test\n\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00";
    static REPEAT_TEST_FILE: &[u8] = b"NUS3\x98\x00\x00\x00AUDIINDX\x04\x00\x00\x00\x02\x00\x00\x00TNID\x08\x00\x00\x00\x00\x00\x00\x00\x01\x00\x00\x00NMOF\x08\x00\x00\x00X\x00\x00\x00f\x00\x00\x00ADOF\x10\x00\x00\x00\x90\x00\x00\x00\x07\x00\x00\x00\x90\x00\x00\x00\x07\x00\x00\x00TNNM\x1c\x00\x00\x00repeat_test_1\x00repeat_test_2\x00JUNK\x0c\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00PACK\x10\x00\x00\x00repeat\n\x00\x00\x00\x00\x00\x00\x00\x00\x00";

    #[test]
    fn attempt_parse_test_file() {
        let nus_file = Nus3audioFile::from_bytes(&TEST_FILE[..]);
        assert_eq!(nus_file.files.len(), 1);
        assert_eq!(nus_file.files[0].name, "test_name");
        assert_eq!(nus_file.files[0].data, b"test\n");
        assert_eq!(nus_file.files[0].id, 69);
    }

    #[test]
    fn attempt_parse_repeat_test_file() {
        let nus_file = Nus3audioFile::from_bytes(&REPEAT_TEST_FILE[..]);
        assert_eq!(nus_file.files.len(), 2);
        assert_eq!(nus_file.files[0].name, "repeat_test_1");
        assert_eq!(nus_file.files[0].data, b"repeat\n");
        assert_eq!(nus_file.files[0].id, 0);
        assert_eq!(nus_file.files.len(), 2);
        assert_eq!(nus_file.files[1].name, "repeat_test_2");
        assert_eq!(nus_file.files[1].data, b"repeat\n");
        assert_eq!(nus_file.files[1].id, 1);
    }

    #[test]
    fn test_write_file() {
        let nus_file = Nus3audioFile::from_bytes(&REPEAT_TEST_FILE[..]);
        let mut nus_write_file: Vec<u8> = Vec::with_capacity(REPEAT_TEST_FILE.len());
        nus_file.write(&mut nus_write_file);
        assert_eq!(REPEAT_TEST_FILE, &nus_write_file[..]);
    }
}
