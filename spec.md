# Inofficial pac format specification

*This is not really an actual spec, but rather a rough explanation on how this format works.*

The pac format was used in Yugioh games on the Nintendo DS. It seems to be optimised for fast
reads, but I don't know the details. The content is padded with `0x00` to align section offsets
to multiples of `0x200`. All numbers are in little endian, meaning that the sequence `42 0A` is
actually interpreted as `0A 42`.

## Header

The header has to sections. A list of file names (name list) contained in the pac file and a list
of offsets (offset list) that maps to the file names. Both of the are aligned to `0x200` and unless
they *both* fit into the first `0x200` bytes of the file they are split up.

### Name list
A name list section starts with two values, an offset for the beginning of the file offset list
(2 bytes) and the number of `0x200`-aligned sections of the name list (1 byte). They are mutually
exclusive; if one is set, the other is set to 0. Let's give them a name: if the first value is set
the header is in Mode 1 and otherwise Mode 2.

The following three examples should cover all edge cases.

`[F0 00] [00]` (Mode 1) would mean that the offset list starts at `0xF0` and that the whole header
fits into the first `0x200` bytes.

`[00 00] [01]` (Mode 2) would mean that there is one section in the name list. This happens if the
offset list causes the total header size to be greater than `0x200`.

`[00 00] [03]` (Mode 2) would mean that there are 3 sections in the name list. The name list itself
is too long to fit into `0x200` bytes.

In Mode 1 the name list always ends on an additional `0x00` byte.

In Mode 2 a new section begins if a file name entry would make the section exceed a length of
`0x200`. Each section starts with the same 3 bytes before the file names are listed and is padded
with `0x00` to be exactly `0x200` bytes long.

The file names have 3 components. The first (1 byte) indicates the number of bytes in the filename.
The second one (1 byte) is a bit odd since it is a checksum that combines all bytes of the
filename with an `XOR`. The reason for that might be error detection, but I'm not too sure either.
The last component is the actual file name (variable length), which has as many bytes as specified
in the first component. There isn't any spacing between the file names since the length suffices
to separate them. All files are listed in sequence.

### Offset list
An offset list section starts with 2 `0x00` bytes for padding, 2 bytes for the number of files in
the section and another 4 bytes of padding. The number of files are either a value smaller than
`0x40` or `FF FF`. The latter implies that the number of files in the section is `0x3F` and that
there is at least one more section. This means that there is always a final section that starts
with a number smaller than `0x40`. `0x3F` is the maximum number of files in a section, because
one more would exceed a size of `0x200`. `FF FF` only ever appears in Mode 2.

Although I call it an offset list, it actually contains two values for each file: an offset from
the first byte that is not part of the header (aligned to `0x200`) and a file size. Both are
4 bytes long.

If a header that takes up the first `0x5FF` bytes the file contents start at `0x600`. Any offset in
the offset list must be added with this value to calculate the absolute offset.

## File contents
Files contents are simply put into the file as is. A file's contents are not split, but still
padded with zeroes if their size isn't an exact multiple of `0x200`.
