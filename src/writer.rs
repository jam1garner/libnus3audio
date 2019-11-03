use super::Nus3audioFile;
use crc::crc32;
use std::collections::HashMap;
use std::mem::size_of;
use binwrite::BinWrite;

fn get_padding_amount(offset: usize) -> usize {
    ((0x18 - (offset as isize % 0x10)) % 0x10) as usize
}

#[derive(BinWrite)]
struct PaddedFile {
    #[binwrite(pad_after(0x10))]
    file: Vec<u8>
}

impl Nus3audioFile {
    pub fn calc_size(&self) -> usize {
        let nus3_size = "NUS3".len() + size_of::<u32>();
        let audi_size = "AUDIINDX".len() + (size_of::<u32>() * 2);
        let tnid_size = "TNID".len() + size_of::<u32>() + (size_of::<u32>() * self.files.len());
        let nmof_size = tnid_size;
        let adof_size = "ADOF".len() + size_of::<u32>() + (size_of::<u32>() * self.files.len() * 2);

        let string_section_start = nus3_size
            + audi_size
            + tnid_size
            + nmof_size
            + adof_size
            + "TNNM".len()
            + size_of::<u32>();

        let mut string_section_size = 0u32;
        for file in self.files.iter() {
            string_section_size += file.name.len() as u32 + 1;
        }

        let junk_pad = get_padding_amount(
            string_section_start + string_section_size as usize + "JUNK".len() + size_of::<u32>(),
        );
        let junk_size = "JUNK".len() + size_of::<u32>() + junk_pad;

        let pack_section_start = string_section_start
            + string_section_size as usize
            + junk_size
            + "PACK".len()
            + size_of::<u32>();

        let mut pack_section_size = 0u32;
        let mut pack_section_size_no_pad = 0u32;
        for file in self.files.iter() {
            pack_section_size_no_pad = pack_section_size + file.data.len() as u32;
            pack_section_size += ((file.data.len() + 0xF) / 0x10) as u32 * 0x10;
        }

        pack_section_start
            + if self.files.len() == 1 {
                pack_section_size_no_pad
            } else {
                pack_section_size
            } as usize
    }

    pub fn write(&self, f: &mut Vec<u8>) {
        // Offset calculation time
        let mut string_offsets: Vec<u32> = vec![];
        let mut file_offsets: Vec<(u32, u32)> = vec![];

        let nus3_size = "NUS3".len() + size_of::<u32>();
        let audi_size = "AUDIINDX".len() + (size_of::<u32>() * 2);
        let tnid_size = "TNID".len() + size_of::<u32>() + (size_of::<u32>() * self.files.len());
        let nmof_size = tnid_size;
        let adof_size = "ADOF".len() + size_of::<u32>() + (size_of::<u32>() * self.files.len() * 2);

        let string_section_start = nus3_size
            + audi_size
            + tnid_size
            + nmof_size
            + adof_size
            + "TNNM".len()
            + size_of::<u32>();

        let mut string_section_size = 0u32;
        for file in self.files.iter() {
            string_offsets.push(string_section_start as u32 + string_section_size);
            string_section_size += file.name.len() as u32 + 1;
        }

        let junk_pad = get_padding_amount(
            string_section_start + string_section_size as usize + "JUNK".len() + size_of::<u32>(),
        );
        let junk_size = "JUNK".len() + size_of::<u32>() + junk_pad;

        let pack_section_start = string_section_start
            + string_section_size as usize
            + junk_size
            + "PACK".len()
            + size_of::<u32>();

        let mut pack_section_size_no_pad = 0u32;
        let mut pack_section_size = 0u32;
        let mut existing_files: HashMap<u32, (u32, u32)> = HashMap::new();
        let mut files_to_pack = vec![];
        for file in self.files.iter() {
            let hash = crc32::checksum_ieee(&file.data);

            let offset_pair = match existing_files.get(&hash) {
                Some(pair) => *pair,
                None => {
                    let pair = (
                        pack_section_start as u32 + pack_section_size,
                        file.data.len() as u32,
                    );
                    existing_files.insert(hash, pair);
                    files_to_pack.push(file);
                    pack_section_size_no_pad = pack_section_size + file.data.len() as u32;
                    pack_section_size += ((file.data.len() + 0xF) / 0x10) as u32 * 0x10;

                    pair
                }
            };
            file_offsets.push(offset_pair);
        }

        if self.files.len() == 1 {
            pack_section_size = pack_section_size_no_pad;
        }

        let filesize = pack_section_start as u32 + pack_section_size;

        // Actually write to file
        &(
            ("NUS3", filesize - nus3_size as u32),
            ("AUDIINDX", 4u32, self.files.len() as u32),
            (
                "TNID",
                self.files.len() as u32 * 4,
                self.files.iter().map(|a| a.id as u32).collect::<Vec<u32>>()
            ),
            (
                "NMOF",
                self.files.len() as u32 * 4,
                string_offsets
            ),
            (
                "ADOF",
                self.files.len() as u32 * 8,
                file_offsets
            ),
            (
                "TNNM",
                string_section_size,
                self.files.iter().map(|file| (&file.name[..], 0u8)).collect::<Vec<_>>()
            ),
            (
                "JUNK", junk_pad as u32, vec![0u8; junk_pad]
            ),
            (
                "PACK", pack_section_size
            )
        ).write(f).unwrap();
        if self.files.len() == 1 {
            BinWrite::write(&self.files[0].data[..], f).unwrap();
        } else {
            BinWrite::write(&files_to_pack, f).unwrap();
        }
    }

    pub fn calc_tonelabel_size(&self) -> usize {
        8 + 8 * self.files.len()
    }

    pub fn write_tonelabel(&self, f: &mut Vec<u8>) {
        let files = &self.files;
        let len = files.len();
        let mut hash_ids = Vec::<u64>::with_capacity(len);

        for audio_file in files.iter() {
            hash_ids.push(
                crc32::checksum_ieee(audio_file.name.to_lowercase().as_bytes()) as u64
                    | (audio_file.name.len() as u64) << 32
                    | (audio_file.id as u64) << 40,
            );
        }

        hash_ids.sort_by(|a, b| (a & 0xffffffffff).partial_cmp(&(b & 0xffffffffff)).unwrap());

        &(
            1u32,
            len as u32,
            hash_ids
        ).write(f);
    }
}

