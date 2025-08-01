use std::{fs::{create_dir_all, read_to_string, write, File, OpenOptions}, io::{Cursor, Error, Read, Write}, path::Path, process::exit};
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};

const CHUNK_SIZE: usize = 0x200;

fn next_chunk(position: usize, chunk_size: usize) -> usize {
    (chunk_size - position % chunk_size) % chunk_size
}

fn xor_hash(data: &[u8]) -> u8 {
    let mut result = 0;
    for c in data { result ^= c; }
    result
}

pub fn unpack(input: &Path, outdir: &Path, verbose: bool) -> Result<(), Error> {
    let mut buf = vec![];
    File::open(input)?.read_to_end(&mut buf)?;
    let mut r = Cursor::new(&buf);
    let mut sizemap_start = r.read_u16::<LittleEndian>()? as usize;
    let namelist_chunks = r.read_u8()? as usize;

    if namelist_chunks > 0 { sizemap_start = namelist_chunks * 0x200; }

    r = Cursor::new(&buf);
    let mut namelist_data = vec![0u8; sizemap_start];
    r.read_exact(&mut namelist_data)?;

    if verbose {
        println!("Size map starts at offset 0x{sizemap_start:X}")
    }

    let mut files = vec![];
    let mut pos;
    for i in 0..namelist_chunks.max(1) {
        pos = 3;
        let off = CHUNK_SIZE * i;

        while pos+off < namelist_data.len() && pos < CHUNK_SIZE {
            let filename_len = namelist_data[pos+off] as usize;
            if filename_len == 0 {
                pos += 1;
                continue;
            }

            pos += 1;
            let expected_hash = namelist_data[pos+off];
            pos += 1;
            let raw_filename = &namelist_data[pos+off .. pos+off+filename_len];
            let filename = match str::from_utf8(raw_filename) {
                Ok(v) => v,
                Err(_) => {
                    eprintln!("Error in {}: failed to decode utf8 string", input.display());
                    exit(1);
                }
            };

            let actual_hash = xor_hash(raw_filename);
            if actual_hash != expected_hash {
                eprintln!("Error in {}: {filename} has hash 0x{actual_hash:X}, but a hash value of 0x{expected_hash:X} was expected", input.display());
                exit(1);
            }
            files.push(filename);
            pos += filename_len;
        }
    }

    let mut remaining_files = files.len();
    let mut positions = vec![];
    loop {
        r.read_u16::<LittleEndian>()?;
        let mut file_count = r.read_u16::<LittleEndian>()? as usize;
        r.read_u32::<LittleEndian>()?;

        let done = file_count != 0xFFFF;
        if !done {
            file_count = 0x3F;
        }

        if file_count != remaining_files.min(0x3F) {
            eprintln!("Error in {}: found {file_count} files but found {} filenames", input.display(), files.len());
            exit(1);
        }

        if verbose {
            println!("Found {file_count} file(s)");
        }

        for i in 0..file_count {
            let offset = r.read_u32::<LittleEndian>()? as usize;
            let size = r.read_u32::<LittleEndian>()? as usize;
            let fname = files[i + files.len() - remaining_files];
            positions.push((fname, offset, size));
            if verbose {
                println!("Found file '{fname}' with offset 0x{offset:X} and size 0x{size:X}");
            }
        }

        remaining_files -= file_count;
        if done { break }
    }

    let data_start = {
        let pos = r.position() as usize;
        pos + next_chunk(pos, CHUNK_SIZE)
    };
    create_dir_all(outdir)?;
    if verbose {
        println!("Data starts at: 0x{data_start:X}");
        println!("Created output directory {}", outdir.display());
    }

    for (name, offset, size) in positions {
        r.set_position((data_start + offset) as u64);
        let mut file_data = vec![0u8; size];
        r.read_exact(&mut file_data)?;
        let outpath = outdir.join(name);
        write(&outpath, file_data)?;
        if verbose {
            println!("Extracted file to {}", outpath.display());
        }
    }

    let pacman_path = outdir.join(format!("{}man", input.file_name().unwrap().to_str().unwrap()));
    let mut pacman = File::create(&pacman_path)?;
    for name in files {
        writeln!(pacman, "{name}")?;
    }

    if verbose {
        println!("Created '{}'", pacman_path.display());
    }

    Ok(())
}

pub fn pack(input: &Path, outdir: &Path, verbose: bool) -> Result<(), Error> {
    let mut fsecs = vec![vec![]];
    let mut files = vec![];

    let pacman_file = read_to_string(input)?;
    for line in pacman_file.lines() {
        let name = line.as_bytes();
        if name.len() > 255 {
            eprintln!("Error: {line} is {} bytes long, but only 255 are allowed", name.len());
            exit(1);
        }

        let name_hash = xor_hash(name);
        let mut file_entry = vec![0u8; 2 + name.len()];
        let mut w = Cursor::new(&mut file_entry);
        w.write_u8(name.len() as u8)?;
        w.write_u8(name_hash)?;
        w.write_all(name)?;
        assert_eq!(file_entry.len(), 2 + name.len());

        if 3 + fsecs.last().unwrap().len() + file_entry.len() >= CHUNK_SIZE {
            let cur_len = fsecs.last().unwrap().len();
            fsecs.last_mut().unwrap().extend(vec![0x00; CHUNK_SIZE - (cur_len + 3)]);
            fsecs.push(vec![]);
        }

        fsecs.last_mut().unwrap().extend(file_entry);
        files.push(line);

        if verbose {
            println!("Added '{line}' (0x{name_hash}) with size 0x{:X}", name.len());
        }
    }

    let mut datalist = vec![vec![]];
    let mut datastr = vec![];
    for name in files {
        let target = input.parent().unwrap().join(name);

        let mut contents = vec![];
        OpenOptions::new().read(true).open(target)?.read_to_end(&mut contents)?;
        if datalist.last().unwrap().len() == 0x3F {
            datalist.push(vec![]);
        }
        datalist.last_mut().unwrap().push((datastr.len(), contents.len()));
        datastr.extend(contents);
        datastr.extend(vec![0; next_chunk(datastr.len(), CHUNK_SIZE)]);
    }

    let mut result = vec![];
    if fsecs.len() == 1 && datalist.len() == 1 && 3 + fsecs[0].len() + datalist[0].len() * 8 + 9 <= CHUNK_SIZE {
        result.write_u16::<LittleEndian>(fsecs[0].len() as u16 + 4)?;
        result.write_u8(0x00)?;
        result.extend(&fsecs[0]);
        result.extend(vec![0; 3]);
        result.write_u16::<LittleEndian>(datalist[0].len() as u16)?;
        result.extend(vec![0; 4]);
        for (a, b) in &datalist[0] {
            result.write_u32::<LittleEndian>(*a as u32)?;
            result.write_u32::<LittleEndian>(*b as u32)?;
        }
    } else {
        for sec in &fsecs {
            result.extend(b"\x00\x00");
            result.write_u8(fsecs.len() as u8)?;
            result.extend(sec);
        }

        result.extend(vec![0; next_chunk(result.len(), CHUNK_SIZE)]);

        for i in 0..datalist.len() {
            result.extend(vec![0; 2]);
            if i+1 == datalist.len() {
                result.write_u16::<LittleEndian>(datalist[i].len() as u16)?;
            } else {
                result.extend(vec![0xFF; 2]);
            }
            result.extend(vec![0; 4]);
            for (a, b) in &datalist[i] {
                result.write_u32::<LittleEndian>(*a as u32)?;
                result.write_u32::<LittleEndian>(*b as u32)?;
            }
        }
    }

    result.extend(vec![0; next_chunk(result.len(), CHUNK_SIZE)]);
    result.extend(datastr);

    create_dir_all(outdir)?;

    let outpac_fname = input.file_name().unwrap().to_str().unwrap();
    let outpac = outdir.join(&outpac_fname[..outpac_fname.len() - 3]);
    write(&outpac, result)?;

    if verbose {
        println!("Created output directory {}", outdir.display());
        println!("Wrote file '{}'", outpac.display());
    }

    Ok(())
}
