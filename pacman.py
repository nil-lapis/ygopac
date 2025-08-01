import os
import struct
import sys

from glob import glob

def xor_hash(data):
    result = 0
    for c in data:
        result ^= ord(c)
    return result

def xor_hash_bytes(data):
    result = 0
    for c in data:
        result ^= c
    return result

def align_to_0x200(size):
    return (size + 0x200) // 0x200 * 0x200

def unpack(path, outdir, verbose=False):
    with open(path, "rb") as f:
        sizelist_start = struct.unpack("<H", f.read(2))[0]
        namelist_parts = struct.unpack("<B", f.read(1))[0]
        if namelist_parts > 0:
            sizelist_start = namelist_parts * 0x200
        namelist_data = 3 * b"\x00" + f.read(sizelist_start - 3)
        if verbose:
            print(f"Size list starts at offset 0x{sizelist_start:x}")

        files = []
        pos = 0
        n = 1

        for i in range(max(1, namelist_parts)):
            pos = 3
            off = 0x200 * i
            while pos+off < len(namelist_data) and pos < 0x200:
                filename_len = namelist_data[pos+off]
                if filename_len == 0:
                    pos += 1
                    continue

                pos += 1
                expected_hash = namelist_data[pos+off]
                pos += 1
                filename = namelist_data[pos+off : pos+off+filename_len].decode("utf8")

                actual_hash = xor_hash(filename)
                if actual_hash != expected_hash:
                    print(f"Error in {path}: {filename} has hash {actual_hash}, but a hash value of {expected_hash} was expected")
                    sys.exit(1)

                files.append(filename)
                pos += filename_len
                n += 1

        remaining_files = len(files)
        positions = []
        while True:
            f.read(2)
            file_count = struct.unpack("<H", f.read(2))[0]
            f.read(4)

            done = True
            if file_count == 0xFFFF:
                done = False
                file_count = 0x3F

            if file_count != min(0x3F, remaining_files):
                print(f"Error in {path}: found {file_count} files but found {len(files)} filenames")
                sys.exit(1)

            if verbose:
                print(f"Found {file_count} file(s)")

            for i in range(file_count):
                offset = struct.unpack("<I", f.read(4))[0]
                size = struct.unpack("<I", f.read(4))[0]
                fname = files[i + len(files) - remaining_files]
                positions.append((fname, offset, size))
                if verbose:
                    print(f"Found file '{fname}' with offset 0x{offset:x} and size 0x{size:x}")

            remaining_files -= file_count

            if done:
                break

        data_start = align_to_0x200(f.tell())
        if verbose:
            print(f"Data starts at: 0x{data_start:x}")

        os.makedirs(outdir, exist_ok=True)
        if verbose:
            print(f"Created output directory {outdir}")

        for (name, offset, size) in positions:
            f.read(data_start + offset - f.tell())
            file_data = f.read(size)
            outpath = os.path.join(outdir, name)
            os.makedirs(os.path.dirname(outpath), exist_ok=True)

            with open(outpath, "wb") as out_f:
                out_f.write(file_data)

            if verbose:
                print(f"Extracted file to {outpath}")

        pacman = os.path.join(outdir, f"{os.path.basename(path)}man")
        with open(pacman, "w") as pac_f:
            for name in files:
                pac_f.write(name + '\n')

        if verbose:
            print(f"Created '{pacman}'")

def pack(input, outdir, verbose=False):
    fsecs = [b""]
    files = []

    with open(input, "rb") as f:
        for line in f.readlines():
            name = line[:-1]
            if len(name) > 255:
                print(f"Error: {name} is {len(name)} bytes long, but only 255 are allowed")
                sys.exit(1)
            name_hash = xor_hash_bytes(name)

            filestr = len(name).to_bytes() + name_hash.to_bytes() + name
            if 3 + len(fsecs[-1]) + len(filestr) >= 0x200:
                fsecs[-1] += b"\x00" * (0x200 - (len(fsecs[-1]) + 3))
                fsecs.append(b"")

            fsecs[-1] += filestr
            files.append(name.decode("utf8"))

            if verbose:
                print(f"Added '{files[-1]}' (0x{name_hash:x}) with size 0x{len(name):x}")

    datalist = [[]]
    datastr = b""
    for name in files:
        target = os.path.join(os.path.dirname(input), name)
        with open(target, "rb") as f:
            content = f.read()
            if len(datalist[-1]) == 0x3F:
                datalist.append([])
            datalist[-1].append((len(datastr), len(content)))
            datastr += content

        if len(datastr) % 0x200 > 0:
            datastr += b"\x00" * (0x200 - len(datastr) % 0x200)

    result = b""
    if len(fsecs) == 1 and len(datalist) == 1 and 3 + len(fsecs[0]) + len(datalist[0]) * 8 + 9 <= 0x200:
        result += struct.pack("<H", len(fsecs[0]) + 4) + b"\x00" + fsecs[0] + 3 * b"\x00"
        result += struct.pack("<H", len(datalist[0]))
        result += 4 * b"\x00"
        for (a, b) in datalist[0]:
            result += struct.pack("<II", a, b)
    else:
        for sec in fsecs:
            result += b"\x00\x00" + struct.pack("<B", len(fsecs)) + sec
        if len(result) % 0x200 > 0:
            result += b"\x00" * (0x200 - len(result) % 0x200)
        for i in range(len(datalist)):
            result += b"\x00\x00"
            if i+1 == len(datalist):
                result += struct.pack("<H", len(datalist[i]))
            else:
                result += b"\xFF\xFF"
            result += 4 * b"\x00"
            for (a, b) in datalist[i]:
                result += struct.pack("<II", a, b)

    if len(result) % 0x200 > 0:
        result += b"\x00" * (0x200 - len(result) % 0x200) + datastr

    os.makedirs(outdir, exist_ok=True)
    if verbose:
        print(f"Created output directory {outdir}")

    outpac = os.path.join(outdir, os.path.basename(input)[:-3])
    with open(outpac, "wb") as f:
        f.write(result)

    if verbose:
        print(f"Wrote file '{outpac}'")

def usage():
    print("Usage:")
    print(f"  Unpack:     python {sys.argv[0]} unpack[_debug] <input.pac> <output_dir>")
    print(f"  Unpack all: python {sys.argv[0]} unpack_all[_debug] <input_dir> <output_dir>")
    print(f"  Pack:       python {sys.argv[0]} pack[_debug] <input.pacman> <output_dir>")
    print(f"  Pack all:   python {sys.argv[0]} pack_all[_debug] <input_dir> <output_dir>")

if __name__ == "__main__":
    if len(sys.argv) != 4:
        usage()
        sys.exit(1)

    cmd = sys.argv[1]
    verbose = False
    if cmd.endswith("_debug"):
        verbose = True
        cmd = cmd[:-6]
    if cmd == "unpack":
        path = sys.argv[2]
        if not os.path.exists(path):
            print(f"Error: pac file '{path}' not found")
            sys.exit(1)
        if not os.path.isfile(path):
            print(f"Error: '{path}' is not a file")
            sys.exit(1)
        if not path.endswith(".pac"):
            print(f"Error: '{path}' is not a .pac file")
            sys.exit(1)
        unpack(path, sys.argv[3], verbose=verbose)
    elif cmd == "unpack_all":
        dir = sys.argv[2]
        if not dir.endswith("/"):
            dir += "/"
        if not os.path.exists(dir):
            print(f"Error: directory '{dir}' not found")
            sys.exit(1)
        if not os.path.isdir(dir):
            print(f"Error: '{dir}' is not a directory")
            sys.exit(1)
        for pat in ("*.pac", "**/*.pac"):
            for path in glob(os.path.join(dir, pat)):
                if verbose:
                    print(f"Unpacking {path}")
                outdir = os.path.join(sys.argv[3], f"{path[len(dir):]}.d")
                unpack(path, outdir, verbose=verbose)
                if verbose:
                    print()
    elif cmd == "pack":
        path = sys.argv[2]
        if not os.path.isfile(path):
            print(f"Error: '{path}' is not a file")
            sys.exit(1)
        if not path.endswith(".pacman"):
            print(f"Error: '{path}' is not a .pacman file")
            sys.exit(1)
        pack(path, sys.argv[3], verbose=verbose)
    elif cmd == "pack_all":
        dir = sys.argv[2]
        if not dir.endswith("/"):
            dir += "/"
        if not os.path.exists(dir):
            print(f"Error: directory '{dir}' not found")
            sys.exit(1)
        if not os.path.isdir(dir):
            print(f"Error: '{dir}' is not a directory")
            sys.exit(1)
        for pat in ("*.pac.d", "**/*.pac.d"):
            for path in glob(os.path.join(dir, pat)):
                if not os.path.isdir(path):
                    print(f"Error: '{path}' is not a directory")
                    sys.exit(1)
                infile = os.path.join(path, os.path.basename(path)[:-2] + "man")
                if not os.path.isfile(infile):
                    print(f"Error: '{infile}' is not a file")
                    sys.exit(1)

                if verbose:
                    print(f"Packing {path}")
                outdir = os.path.join(sys.argv[3], os.path.dirname(path)[len(dir):])
                pack(infile, outdir, verbose=verbose)
                if verbose:
                    print()
    else:
        print(f"Error: unknown command `{cmd}`")
        print()
        usage()
        sys.exit(1)
