# SqPack Format

This document is to better document the procedure for extracting simple table data from FFXIV's sqpack files. It's not intended to be a complete deep dive into the internals, but mainly to piece together the various aspects I've gleaned from the various programs out there.

## Intro

### What are we doing?

So let's say you want to read the items table in the game. How do? First off, one needs to specifically know the table name -- in this case `item`. How does one figure out what table names exist in the sqpack file? Not 100% sure, but they exist as strings inside the FFXIV executable file. For now, I'll just say "check the csv file names from XIVAPI."

Once we have the table name that we want to look up, we need to create a file path for us to look up. In this case, table data first must be found by looking for the `exd/{table_name}.exh` file.

Our first task then, is to fetch the contents of the `exd/item.exh` file.

### SqPack Files

FFXIV stores its many resource files in a set of compressed sqpack files of the form `{FFXIV-Game-Directory}/sqpack/{expansion}/{type}.win32.*`.

In order to look up the `exd/item.exh` file in these, we first need to determine a few things:

1. Which sqpack file will we use?
1. What is the hash of the searched-for file path?

#### Which file do we look into?

For the first consideration, there're different type files for different formats that are better documented elsewhere, but lucky for us, `exd` files are always found in the `sqpack/ffxiv/0a0000.win32.*` files.

In this case, there are three files:

* `sqpack/ffxiv/0a0000.win32.dat0`
* `sqpack/ffxiv/0a0000.win32.index`
* `sqpack/ffxiv/0a0000.win32.index2`

#### What is the file path hash?

We're going to be looking for the following file: `exd/item.exh`. As far as I know, this path isn't stored anywhere in the sqpack files. However, the index files function as a table of contents for the main `dat` files. However, the full path isn't stored, but rather a pair of hashes. How are these hashes calculated?

1. Convert the path to lower case (`exd/item.exh`)
1. Split the path into two components: directory (`exd`) and file name (`item.exh`)
1. For each component, calculate the CRC32 value using the JAMCRC algorithm. This is just the bitwise inverse of the more-typical ISO-HDLC algorithm.

| path | crc32 | jam_crc32 |
| - | - | - |
| `exd` | `0x1C648666` | `0xE39B7999` |
| `item.exh` | `0x21447686` | `0xDEBB8979` |

Now that we have the two JAMCRC hashes, we may define our final hash as the simple concatenation of the two:

* `ffxiv_hash('exd/item.exh') == 0xE39B7999_DEBB8979`.

## Index File Parsing

Now that we have the hash-key for the file path, we need to read the table of contents from the index file. Let's take a look at a hex dump of `0a0000.win32.index`. Up first the general header:

### General SqPack Header

| Offset | `00 01 02 03 04 05 06 07 08 09 0A 0B 0C 0D 0E 0F` |
| - | - |
| **`0x0000`** | `53 71 50 61 63 6B 00 00 00 00 00 00 00 04 00 00` |
| **`0x0010`** | `01 00 00 00 02 00 00 00 00 00 00 00 00 00 00 00` |

I'll split this into a few segments. All numerical values are parsed here in little-endian format:

| Data | Value | Description |
| - | - | - |
| `53 71 50 61 63 6B 00 00` | `SqPack` | Magic Number that all sqpack files should begin with. Zero-padded ASCII |
| `00 00 00 00` | `0` | Platform ID. Zero is `win32` |
| `00 40 00 00` | `0x400` | Header size |
| `01 00 00 00` | `1` | Version |
| `02 00 00 00` | `2` | File Type |

The total space of the general header is `0x400` bytes, so let's now move to the table of contents section, immediately afterwards.

### Table of Contents Header

Here's a dump of the next section of the `0a0000.win32.index` file, the Table of Contents header:

| Offset |  `00 01 02 03 04 05 06 07 08 09 0A 0B 0C 0D 0E 0F` |
| - | - |
| **`0x0400`** | `00 04 00 00 01 00 00 00 00 08 00 00 30 AD 08 00` |

This is all that we'll need for our immediate purposes:

| Data | Value | Description |
| - | - | - |
| `00 04 00 00` | `0x400` | Header Size |
| `01 00 00 00` | `1` | Version |
| `00 08 00 00` | `0x800` | Data Offset |
| `30 AD 08 00` | `0x8AD30` | Data Size |

Now, we know the offset with which we'll find our ToC data entries. Let's read them in:

### Table of Contents entry

This concerns the `*.win32.index` file entries, rather than the `index2` files, which are slightly different in the way they use their hashes.

#### The first ToC entry

Let's dump the first chunk of the `0a0000.win32.index` file from this offset:

| Offset | `00 01 02 03 04 05 06 07 08 09 0A 0B 0C 0D 0E 0F` |
| - | - |
| **`0x0800`** | `5B FD FA 01 85 AE A4 00 B0 C2 9C 01 00 00 00 00` |

Considering this row, we have:

| Data | Value | Description |
| - | - | - |
| `5B FD FA 01 85 AE A4 00` | `0x00A4AE85_01FAFD5B` | Hash Key |
| `B0 C2 9C 01` | `0x019CC2B0` | Data |
| `00 00 00 00` | -- | Empty Padding |

This data is a bit-packed value that contains three fields.

| Size (Bits) | Description |
| - | - |
| `1` | Unknown |
| `3` | `dat` file ID |
| `28` | Chunk Offset |

Splitting up the data value, we get:

| Value | Description |
| - | - |
| `0b0` | Unknown|
| `0b000` | `dat` file ID |
| `0x19CC2B` | Chunk Offset |

To convert from a chunk offset to file offset as bytes, we multiply this by `0x80`, so: `file_offset = 0x19CC2B * 0x80 = 0xCE61580`.

Thus, this Table of Contents entry is:

| Hash | File ID | File Offset |
| - | - | - |
| `0x00A4AE85_01FAFD5B` | `0` | `0xCE61580` |

#### Differences between index & index2 files

The `index2` files change things as follows:

* The `Hash` value of path to lookup is no-longer split between the directory and file name, but rather combined as one path.
* There is no end-padding. The full length of a ToC entry in the index2 files is a `U32` `Hash` and a `U32` `Data` field (identically treated as above) for a total of 8 bytes.

#### The Item table entry

We can continue scanning through the ToC entries until we find the one we're looking for. For a reminder, we had the following hash key from earlier: `ffxiv_hash('exd/item.exh') == 0xE39B7999_DEBB8979`.

| Offset | `00 01 02 03 04 05 06 07 08 09 0A 0B 0C 0D 0E 0F` |
| - | - |
| **`0x71A90`** | `79 89 BB DE 99 79 9B E3 00 6F 17 00 00 00 00 00` |

This corresponds to an entry of:

| Data | Value | Description |
| - | - | - |
| `79 89 BB DE 99 79 9B E3` | `0xE39B7999_DEBB8979` | Hash Key |
| `00 6F 17 00` | `0x00176F00` | Data |
| `00 00 00 00` | -- | Empty Padding |

Splitting up the data value (`0x00176F00`), we get:

| Value | Description |
| - | - |
| `0b0` | Unknown|
| `0b000` | `dat` file ID |
| `0x176F0` | Chunk Offset |

Multiplying the chunk offset by `0x80`, we get an absolute file offset of: `file_offset = 0x176F0 * 0x80 = 0xBB7800`

Or, the full data for the ToC entry for `exd/item.exh`, that we will subsequently look up in the data file:

| Hash | File ID | File Offset |
| - | - | - |
| `0xE39B7999_DEBB8979` | `0` | `0xBB7800` |

## Data File Block Parsing

Now that we have the information for finding `exd/item.exh`, we can take a look inside the `0a0000.win32.dat0` file. The `0` at the end should match the File ID of the ToC entry.

### Block Information Header

| Offset | `00 01 02 03 04 05 06 07 08 09 0A 0B 0C 0D 0E 0F` |
| - | - |
| **`0xBB7800`** | `80 00 00 00 02 00 00 00 60 04 00 00 05 00 00 00` |
| **`0xBB7810`** | `05 00 00 00 01 00 00 00 00 00 00 00 80 02 60 04` |

This is the start of the block information header. Here's how it may be parsed:

| Data | Value | Description |
| - | - | - |
| `80 00 00 00` | `0x80` | Header Size |
| `02 00 00 00` | `2` | File Type |
| `60 04 00 00` | `0x460` | File Size |
| `05 00 00 00` | `5` | Number of Blocks? (unimportant for us) |
| `05 00 00 00` | `5` | Block Buffer Size? (unimportant for us) |
| `01 00 00 00` | `1` | Block Count |

Possible values for File Type are:

| Value | Description |
| - | - |
| `1` | Empty |
| `2` | Standard |
| `3` | Model |
| `4` | Texture |

I'll only be covering Standard files here. Subsequently in the file, each block (of Block Count) is then read:

| Offset | `00 01 02 03 04 05 06 07 08 09 0A 0B 0C 0D 0E 0F` |
| - | - |
| **`0xBB7800`** | ~~`80 00 00 00 02 00 00 00 60 04 00 00 05 00 00 00`~~ |
| **`0xBB7810`** | ~~`05 00 00 00 01 00 00 00`~~ `00 00 00 00 80 02 60 04` |

Since there's a Block Count of 1, it is as follows:

| Data | Value | Description |
| - | - | - |
| `00 00 00 00` | `0` | Offset |
| `80 02` | `0x280` | Compressed Block Size |
| `60 04` | `0x460` | Uncompressed Block Size |

### Block Data Information

Now we can skip past the rest of the header (which was of size `0x80`), and move to the section of the file for the first block (offset `0`, size `0x280`).

| Offset | `00 01 02 03 04 05 06 07 08 09 0A 0B 0C 0D 0E 0F` |
| - | - |
| **`0xBB7880`** | `10 00 00 00 00 00 00 00 1B 02 00 00 60 04 00 00` |

This corresponds to:

| Data | Value | Description |
| - | - | - |
| `10 00 00 00` | `0x10` | Data Header Size |
| `00 00 00 00` | -- | Unknown |
| `1B 02 00 00` | `0x21B` | Compressed Data Size |
| `60 04 00 00` | `0x460` | Uncompressed Data Size |

Subsequently is `0x21B` bytes of compressed data:

| Offset | `00 01 02 03 04 05 06 07 08 09 0A 0B 0C 0D 0E 0F` |
| - | - |
| **`0xBB7890`** | `4D D4 4D 48 D3 01 1C C6 F1 DF 9A 73 86 4A BE 92` |
| \[...\] | \[...\] |
| **`0xBB7AA0`** | `01 FF 18 8C F8 A2 9B F0 47 7D 0C 00` ~~`00 00 00 00`~~ |

We can take this block of data, and send it through a DEFLATE decompressor to finally retrieve the original `exd/item.exh` file.

## Excel Header File Format

Now that we've deflated the `0x21B` bytes into `0x460` uncompressed bytes, let's take a look at our extracted `exd/item.exh` file.

***IMPORTANT NOTE:*** All numbers inside this file are read BigEndian, not LittleEndian like before.

### Header Information

The decompressed `exd/item.exh` data:

| Offset | `00 01 02 03 04 05 06 07 08 09 0A 0B 0C 0D 0E 0F` |
| - | - |
| **`0x0000`** | `45 58 48 46 00 03 00 A0 00 5B 00 59 00 06 42 58` |
| **`0x0010`** | `00 01 00 00 00 00 AD 71 00 00 00 00 00 00 00 00` |

Information inside is:

| Data | Value | Description |
| - | - | - |
| `45 58 48 46` | `EXHF` | Magic Number |
| `00 03` | `3` | Unknown |
| `00 A0` | `0xA0` | Data Offset |
| `00 5B` | `0x5B` (`91`) | Column Count |
| `00 59` | `0x59` (`89`) | Page Count |
| `00 06` | `6` | Language Count |
| `42 58` | -- | Unknown |
| `00` | `0` | U2 (no idea what this does) |
| `01` | `1` | Variant Type |
| `00 00` | `0` | U3 (no idea what this does) |
| `00 00 AD 71` | `0xAD71` (`44401`) | Row Count |
| `00 00 00 00` `00 00 00 00` | `0` `0` | U4\[2\] (no idea what this does) |

The `Data Offset` value will be important later. The variant type should be one of the following values:

| Value | Description |
| - | - |
| `1` | Default |
| `2` | SubRows |

SubRows has a different method of parsing, but none of the tables I've looked at yet, deal with it, so I don't know the details.

### Column Information

Immediately after the header, the next `Column Count` entry of column data consists of:

| Offset | `00 01 02 03 04 05 06 07 08 09 0A 0B 0C 0D 0E 0F` |
| - | - |
| **`0x0020`** | `00 00 00 00 00 02 00 10 00 00 00 04 00 02 00 11` |
| \[...\] | \[...\] |

Let's check the data of the first few columns (out of `91` total, as decoded above):

| Data | Value | Description |
| - | - | - |
| `00 00` | `0` | Data Type #1 |
| `00 00` | `0x00` | Offset #1 |
| `00 02` | `2` | Data Type #2 |
| `00 10` | `0x10` | Offset #2 |
| `00 00` | `0` | Data Type #3 |
| `00 04` | `0x04` | Offset #3 |
| `00 02` | `2` | Data Type #4 |
| `00 11` | `0x11` | Offset #4 |
| \[...\] | \[...\] | \[...\] |

The Data Type field may be one of the following values:

| Value | Description |
| - | - |
| `0x00` | String |
| `0x01` | Bool |
| `0x02` | Int8 |
| `0x03` | UInt8 |
| `0x04` | Int16 |
| `0x05` | UInt16 |
| `0x06` | Int32 |
| `0x07` | UInt32 |
| `0x09` | Float32 |
| `0x0A` | Int64 |
| `0x0B` | UInt64 |
| `0x19` | PackedBool0 |
| `0x1A` | PackedBool1 |
| `0x1B` | PackedBool2 |
| `0x1C` | PackedBool3 |
| `0x1D` | PackedBool4 |
| `0x1E` | PackedBool5 |
| `0x1F` | PackedBool6 |
| `0x20` | PackedBool7 |

So, the first few column types read from the `exd/item.exh` data sheet are:

| - | - | - | - | - | - | - | - | - | - | - | - |
| - | - | - | - | - | - | - | - | - | - | - | - |
| **Offset** | `0x00` | `0x10` | `0x04` | `0x11` | `0x12` | `0x13` | `0x14` | `0x15` | `0x08` | `0x0C` | \[...\] |
| **Type** | String | Int8 | String | Int8 | Int8 | Int8 | Int8 | Int8 | String | String | \[...\] |
| - | - | - | - | - | - | - | - | - | - | - | - |

### Page Data

After the column data inside `exd/item.exh`, we find information about the pages for this data. Each table is divided into multiple pages, where each page has many rows. As we decoded in the header, there are `89` pages for this table.

| Offset | `00 01 02 03 04 05 06 07 08 09 0A 0B 0C 0D 0E 0F` |
| - | - |
| \[...\] | \[...\] |
| **`0x0180`** | ~~`00 1A 00 69 00 03 00 68 00 1B 00 69`~~ `00 00 00 00` |
| **`0x0190`** | `00 00 01 F4 00 00 01 F4 00 00 01 F4 00 00 03 E8` |
| **`0x01A0`** | `00 00 01 F4 00 00 05 DC 00 00 01 F4 00 00 07 D0` |
| \[...\] | \[...\] |

Here's a breakdown of the first few page data segments:

| Data | Value | Description |
| - | - | - |
| `00 00 00 00` | `0x000` (`0`) | Page 0 StartingRow |
| `00 00 01 F4` | `0x1F4` (`500`) | Page 0 RowCount |
| `00 00 01 F4` | `0x1F4` (`500`) | Page 1 StartingRow |
| `00 00 01 F4` | `0x1F4` (`500`) | Page 1 RowCount |
| `00 00 03 E8` | `0x3E8` (`1000`) | Page 2 StartingRow |
| `00 00 01 F4` | `0x1F4` (`500`) | Page 2 RowCount |
| `00 00 05 DC` | `0x5DC` (`1500`) | Page 3 StartingRow |
| `00 00 01 F4` | `0x1F4` (`500`) | Page 3 RowCount |
| \[...\] | \[...\] | \[...\] |

### Languages

Finally, after the page data, the language information is found. As decoded above, we read all `6` of the language values from the end of the file. Perplexingly, these are all stored in little-endian format:

| Offset | `00 01 02 03 04 05 06 07 08 09 0A 0B 0C 0D 0E 0F` |
| - | - |
| \[...\] | \[...\] |
| **`0x0450`** | ~~`00 00 01 91`~~ `01 00 02 00 03 00 04 00 05 00 07 00` |

Languages in `exd/item.exh`:

| Data | Value | Description |
| - | - | - |
| `01 00` | `1` | Japanese |
| `02 00` | `2` | English |
| `03 00` | `3` | German |
| `04 00` | `4` | French |
| `05 00` | `5` | Simplified Chinese |
| `07 00` | `7` | Korean |

The other possible values are: `0` (None) and `6` (Traditional Chinese).

## Excel Data File Format

Now that we've extracted page information from the `exd/item.exh` file, we can fetch files for each page of the table.

Each page is located in the following path format: `exd/{table_name}_{startRowId}{languageCode}.exd`

In this scenario, the first few pages of the `item` table are found in the following files: `exd/item_0_en.exd`, `exd/item_500_en.exd`, `exd/item_1000_en.exd`, `exd/item_1500_en.exd`, etc.

Some tables do not have language data, for example the `recipe` table, and so the file for its data is `exd/recipe_0.exd`.

### Excel Data File Header

Let's take a look at the header contents of `exd/item_0_en.exd` after it's extracted like we did for `exd/item.exh`:

| Offset | `00 01 02 03 04 05 06 07 08 09 0A 0B 0C 0D 0E 0F` |
| - | - |
| **`0x0000`** | `45 58 44 46 00 02 00 00 00 00 0F A0 00 01 DD E8` |
| **`0x0010`** | `00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00` |
| **`0x0020`** | `00 00 00 00 00 00 0F C0 00 00 00 01 00 00 10 6C` |
| \[...\] | \[...\] |

Let's break down the values included:

| Data | Value | Description |
| - | - | - |
| `45 58 44 46` | `EXDF` | Magic |
| `00 02` | `2` | Version |
| `00 00` | `0` | ?? |
| `00 00 0F A0` | `0xFA0` (`4000`) | Row Info Size |

This `Row Info Size` value is the only thing of note here, and may be divided by 8 to determine the number of rows in this page file. In this case, `RowCount = RowInfoSize / 8 = 4000 / 8 = 500` rows.

### Row Information

After the header, we can then pull `RowCount` number of row data from the file, starting at address `0x0020`:

| Offset | `00 01 02 03 04 05 06 07 08 09 0A 0B 0C 0D 0E 0F` |
| - | - |
| **`0x0020`** | `00 00 00 00 00 00 0F C0 00 00 00 01 00 00 10 6C` |
| **`0x0030`** | `00 00 00 02 00 00 11 3C 00 00 00 03 00 00 12 40` |
| \[...\] | \[...\] |

Here's what the first few row data chunks looks like:

| Data | Value | Description |
| - | - | - |
| `00 00 00 00` | `0` | Row 0 ID |
| `00 00 0F C0` | `0xFC0` | Row 0 Offset |
| `00 00 00 01` | `1` | Row 1 ID |
| `00 00 10 6C` | `0x106C` | Row 1 Offset |
| `00 00 00 02` | `2` | Row 2 ID |
| `00 00 11 3C` | `0x113C` | Row 2 Offset |
| `00 00 00 03` | `3` | Row 3 ID |
| `00 00 12 40` | `0x1240` | Row 3 Offset |
| \[...\] | \[...\] | \[...\] |

### Row Data Format

Now that we know the offsets for each row, we can read each row's data. We'll first need to collect some information to read the row data. We'll need the original header file's `Data Offset` value (`0xA0`). And we'll need the column information for the table:

| - | - | - | - | - | - | - | - | - | - | - | - |
| - | - | - | - | - | - | - | - | - | - | - | - |
| **Offset** | `0x00` | `0x10` | `0x04` | `0x11` | `0x12` | `0x13` | `0x14` | `0x15` | `0x08` | `0x0C` | \[...\] |
| **Type** | String | Int8 | String | Int8 | Int8 | Int8 | Int8 | Int8 | String | String | \[...\] |
| - | - | - | - | - | - | - | - | - | - | - | - |

Because the first entry in the `item` table is weird, we'll look at the second:

| Value | Description |
| - | - | - |
|`1` | Row 1 ID |
| `0x106C` | Row 1 Offset |

#### Row Data Header

Let's now take a look at the `exd/item_0_en.exd` file at this offset:

| Offset | `00 01 02 03 04 05 06 07 08 09 0A 0B 0C 0D 0E 0F` |
| - | - |
| **`0x1060`** | ~~`00 02 00 02 00 00 00 00 00 00 00 00`~~ `00 00 00 CA` |
| **`0x1070`** | `00 01 00 00 00 00 00 00 00 04 00 00 00 08 00 00` |

The table row will start with two properties of the row:

| Data | Value | Description |
| - | - | - |
| `00 00 00 CA` | `0xCA` | Row Full-Byte Size |
| `00 01` | `1` | Number of Rows |

#### Row Cell Data

Then, starting at `0x1072` we begin to read the cells for each column of the row:

| Offset | `00 01 02 03 04 05 06 07 08 09 0A 0B 0C 0D 0E 0F` |
| - | - |
| **`0x1070`** | ~~`00 01`~~ `00 00 00 00 00 00 00 04 00 00 00 08 00 00` |
| \[...\] | \[...\] |
| **`0x1100`** | `00 00 00 00 00 00 00 00 01 10 3F 00 00 03 00 02` |
| **`0x1110`** | `00 00 67 69 6C 00 67 69 6C 00 53 74 61 6E 64 61` |
| **`0x1120`** | `72 64 20 45 6F 72 7A 65 61 6E 20 63 75 72 72 65` |
| **`0x1130`** | `6E 63 79 2E 00 47 69 6C 00 00 00 00 00 00 00 FE` |
| \[...\] | \[...\] |

Our first column is of type `String`, which is not a simple data class, so the we'll perform a special step:

* Read `StringOffset` = `00 00 00 00` = `0` from the stream. This will be the offset for the post-data section of this row.
* Calculate the offset of the post-data section as follows:
  * `StringDataOffset = StartOffset + DataOffset + StringOffset`
  * `StringDataOffset = 0x1072 + 0xA0 + 0 = 0x1112`

Starting at address `0x1112` we read bytes until we reach `0x00`: `67 69 6C 00`. These are the ASCII characters for the C-string '`gil`'. So, the first column's data is `String('gil')`.

Looking now at the data, we see:

| Offset | `00 01 02 03 04 05 06 07 08 09 0A 0B 0C 0D 0E 0F` |
| - | - |
| **`0x1070`** | ~~`00 01 00 00 00 00`~~ `00 00 00 04 00 00 00 08 00 00` |
| **`0x1080`** | `00 23 00 00 00 01 00 00 00 00 00 00 00 00 00 00` |
| \[...\] | \[...\] |
| **`0x1100`** | `00 00 00 00 00 00 00 00 01 10 3F 00 00 03 00 02` |
| **`0x1110`** | `00 00 67 69 6C 00 67 69 6C 00 53 74 61 6E 64 61` |
| **`0x1120`** | `72 64 20 45 6F 72 7A 65 61 6E 20 63 75 72 72 65` |
| **`0x1130`** | `6E 63 79 2E 00 47 69 6C 00 00 00 00 00 00 00 FE` |
| \[...\] | \[...\] |

The second column has an offset of `0x10` and is of type `Int8`, so we simply read a single signed-byte from `StartOffset + ColumnOffset = 0x1072 + 0x10 = 0x1082`. This second column value is thus `Int8(0)`

The third column has an offset of `0x04` and is another `String` value. So, we read the offset from `0x1072 + 0x4 = 0x1076`, which has a value of `00 00 00 04 = 0x04`, find the string offset as: `StringDataOffset = 0x1072 + 0xA0 + 0x04 = 0x1116` and read bytes from `0x1116` until we hit a null value. `67 69 6C 00` again, or `String('gil')`.

Most types are straight forward to read, e.g. `Uint32`, `Int16`, etc.

There's only one other type of value that we haven't covered here, and let's move forward to the 22nd-25th column:

| - | - | - | - | - | - |
| - | - | - | - | - | - |
| **Offset** | \[...\] | `0x9F` | `0x9F` | `0x9F` | `0x9F` |
| **Type** | \[...\] | `PackedBool0` | `PackedBool1` | `PackedBool2` | `PackedBool3` |
| - | - | - | - | - | - |

These `PackedBoolN`s are simply checking the `N`th bit of the byte found at the offset. So, let's find the absolute offset by taking the `Offset = StartOffset + ColumnOffset = 0x1072 + 0x9F = 0x1111`.

This byte is `0x00`, or `0b0000_0000`. In this scenario, the four lowest bits (on the right) are all `Bool(false)`, but if for example, the byte's value was `0x0B` or `0b0000_1011`, then the values of these columns would be:

| Type | Mask | Value |
| - | - | - |
| `PackedBool0` | `0b1011 & 0b0001 = 0b0001` | `true` |
| `PackedBool1` | `0b1011 & 0b0010 = 0b0010` | `true` |
| `PackedBool2` | `0b1011 & 0b0100 = 0b0000` | `false` |
| `PackedBool3` | `0b1011 & 0b1000 = 0b1000` | `true` |

Now that we've covered all types, we can now read each row of the table.

## Summary

I'll now recount the steps we took to read the `item` table.

* Extract the `exd/item.exh` file
  * Calculate the CRC hash for the `exd/item.exh`.
  * Find a match in the `0a0000.win32.index` file's table of contents.
  * Using the offset calculated from the ToC, read the block information for the `exd/item.exh` file, contained within the `0a0000.win32.dat0` file.
  * Extract the compressed data for the `exd/item.exh` file and run it through a DEFLATE algorithm.
* Read the contents of `exd/item.exh`
  * Read the `DataOffset` value, the column types and the page information settings.
* Using the page information, read the contents of `exd/item_0_en.exd` through `exd/item_44000_en.exd` (in 500 increments).
  * Read the header and gather the row information.
  * For each row, read each cell based off of the column type.

And with that, you should be able to read most of the tables in the FFXIV data files.
