
use a2::Peripheral;
use std::io::File;

static NUM_DRIVES: uint = 2;
static NUM_TRACKS: uint = 35;
static RAW_TRACK_SIZE: uint = 0x1a00;
static RAW_SECTOR_SIZE: uint = 383;
static SECTOR_SIZE: uint = 256;
static SECTORS_PER_TRACK: uint = 16;

type RawTrackData = [u8, ..RAW_TRACK_SIZE];
type RawDiskData = [RawTrackData, ..NUM_TRACKS];

type TrackImage = [u8, ..SECTORS_PER_TRACK*SECTOR_SIZE];
type DiskImage = [TrackImage, ..NUM_TRACKS];

struct Drive
{
   disk_data: RawDiskData,		 // disk data
   track_data: RawTrackData,	// array of track data
   track: uint,			// current track # 
   track_index: uint,		 // position of read head along track
}

static NoDisk: Option<Drive> = None;

pub struct DiskController
{
   drives: [Option<Drive>, ..NUM_DRIVES],
   selected: u8,		// selected drive (0 or 1)
   motor: bool,		// is motor on?
   read_mode: bool,
   write_protect: bool,
}

impl DiskController
{
   pub fn new() -> DiskController { DiskController {
      drives: [NoDisk, NoDisk],
      selected: 0,
      motor: false,
      read_mode: false,
      write_protect: false,
   } }
   
   pub fn load_disk(&mut self, disknum: int, imagefilename: &str)
   {
      let mut f = File::open(&Path::new(imagefilename));
      let mut disk_image : DiskImage = [[0, ..SECTORS_PER_TRACK*SECTOR_SIZE], ..NUM_TRACKS];
      let mut disk_data : RawDiskData = [[0, ..RAW_TRACK_SIZE], ..NUM_TRACKS];
      for track in range(0, NUM_TRACKS)
      {
         // TODO: check length?
         f.read(disk_image[track]);
         disk_data[track] = nibblizeTrack(254, track as u8, disk_image);
      }
      self.drives[disknum] = Some(Drive {
         disk_data: disk_data,
         track_data: [0, ..RAW_TRACK_SIZE],
         track: 0,
         track_index: 0,
      });
      debug!("loaded disk image {}", imagefilename);
                  
   }

//   fn drive<'r>(&'r self) -> &'r Option<~Drive> { &self.drives[self.selected] }
}

impl Drive
{
   fn read_latch(&mut self) -> u8
   {
      debug!("read latch @ {:x} track {}", self.track_index, self.track)
      self.track_index = (self.track_index + 1) % RAW_TRACK_SIZE;
      return self.track_data[self.track_index];
   }

   fn write_latch(&mut self, value: u8)
   {
      debug!("write latch @ {:x} track {}", self.track_index, self.track)
      self.track_index = (self.track_index + 1) % RAW_TRACK_SIZE;
      self.track_data[self.track_index] = value;
   }

   fn servo_phase(&mut self, phase: uint)
   {
      let mut new_track = self.track;

      // if new phase is even and current phase is odd
      if (phase == ((new_track - 1) & 3))
      {
         if (new_track > 0)
         {
            new_track -= 1;
         }
      } else if (phase == ((new_track + 1) & 3))
      {
         if (new_track < NUM_TRACKS*2-1)
         {
            new_track += 1;
         }
      }
      if ((new_track & 1) == 0)
      {
         self.track_data = self.disk_data[new_track>>1];
      } else {
         // TODO: self.track_data = None;
      }
      self.track = new_track;
      debug!("phase {:x} track = {}", phase, self.track as f32*0.5);
   }
}

impl Peripheral for DiskController
{

/*
 * Implement the Disk II softswitches that perform the same function whether
 * they are read or written to.
 */
   fn doHighIO(&mut self, addr: u16, val: u8) -> u8
   {
      PROM[addr & 0xff]
   }

   fn doIO(&mut self, addr: u16, val: u8) -> u8
   {
      //debug!("disk IO {:x} -> {:x}", addr, val);
      let &mut drive = &self.drives[self.selected];
      match addr & 0xf
      {
         /*
          * Turn motor phases 0 to 3 on.  Turning on the previous phase + 1
          * increments the track position, turning on the previous phase - 1
          * decrements the track position.  In this scheme phase 0 and 3 are
          * considered to be adjacent.  The previous phase number can be
          * computed as the track number % 4.
          */
         1|3|5|7 if drive.is_some() => { drive.unwrap().servo_phase(((addr>>1) & 3) as uint); }
            /*
             * Turn drive motor off.
             */
         8 => { self.motor = false; }
            /*
             * Turn drive motor on.
             */
         9 => { self.motor = true; }
            /*
             * Select drive 1.
             */
         0xa => { self.selected = 0; }
            /*
             * Select drive 2.
             */
         0xb => { self.selected = 1; }
            /*
             * Select write mode.
             */
         0xf => { self.read_mode = false; }
            /*
             * Read a disk byte if read mode is active.
             */
         0xc if self.read_mode && drive.is_some() => { return drive.unwrap().read_latch(); }
            /*
             * Select read mode and read the write protect status.
             */
         0xe => { self.read_mode = true; }
            /*
             * Write a disk byte if write mode is active and the disk is not
             * write protected.
             */
         0xd if !self.read_mode && !self.write_protect && drive.is_some() => { drive.unwrap().write_latch(val); }
            /*
             * Read the write protect status only.
             */
         0xd if self.write_protect => { return 0x80; }
         0xd => { return 0; }
         _ => { return 0; }
      }
      return 0; //emu.noise();
   }


}

/* --------------- TRACK CONVERSION ROUTINES ---------------------- */
/*
 * Encode a 256-byte sector as SECTOR_SIZE disk bytes as follows:
 *
 *   14 sync bytes
 *   3 address header bytes
 *   8 address block bytes
 *   3 address trailer bytes
 *   6 sync bytes
 *   3 data header bytes
 * 343 data block bytes
 *   3 data trailer bytes
 */
   fn nibblizeSector(vol: u8, trk: u8, sector: u8, bytes: &[u8]/*[u8, ..256]*/) -> ~[u8] //[u8, ..RAW_SECTOR_SIZE]
   {
      assert!(bytes.len() == 256);
      /*
       * Step 1: write 6 sync bytes (0xff's).  Normally these would be
       * written as 10-bit bytes with two extra zero bits, but for the
       * purpose of emulation normal 8-bit bytes will do, since the
       * emulated drive will always be in sync.
       */

      /*
       * Step 2: write the 3-byte address header (0xd5 0xaa 0x96).
       */

      /*
       * Step 3: write the address block.  Use 4-and-4 encoding to convert
       * the volume, track and sector and checksum into 2 disk bytes each.
       * The checksum is a simple exclusive OR of the first three values.
       */
      let chksum = vol^trk^sector;
      let address_block = [
         ((vol >> 1) | 0xaa), (vol | 0xaa),
         ((trk >> 1) | 0xaa), (trk | 0xaa),
         ((sector >> 1) | 0xaa), (sector | 0xaa),
         ((chksum >> 1) | 0xaa), (chksum | 0xaa)
      ];

      /*
       * Step 4: write the 3-byte address trailer (0xde 0xaa 0xeb).
       */

      /*
       * Step 5: write another 6 sync bytes.
       */

      /*
       * Step 6: write the 3-byte data header.
       */

      /*
       * Step 7: read the next 256-byte sector from the old disk image file,
       * and add two zero bytes to bring the number of bytes up to a multiple
       * of 3.
       */
      let sector_buffer = [bytes.to_owned(), ~[0u8, ..2]].concat_vec();

      /*
       * Step 8: write the first 86 disk bytes of the data block, which
       * encodes the bottom two bits of each sector byte into six-bit
       * values as follows:
       *
       * disk byte n, bit 0 = sector byte n,       bit 1
       * disk byte n, bit 1 = sector byte n,       bit 0
       * disk byte n, bit 2 = sector byte n +  86, bit 1
       * disk byte n, bit 3 = sector byte n +  86, bit 0
       * disk byte n, bit 4 = sector byte n + 172, bit 1
       * disk byte n, bit 5 = sector byte n + 172, bit 0
       *
       * The scheme allows each pair of bits to be shifted to the right out
       * of the disk byte, then shifted to the left into the sector byte.
       *
       * Before the 6-bit value is translated to a disk byte, it is exclusive
       * ORed with the previous 6-bit value, hence the values written are
       * really a running checksum.
       */
      let mut prev_value = 0;
      let mut value = 0;
      let mut data_block_1 = [0, ..86];
      for i in range(0,86)
      {
         value  = (sector_buffer[i] & 0x01) << 1;
         value |= (sector_buffer[i] & 0x02) >> 1;
         value |= (sector_buffer[i + 86] & 0x01) << 3;
         value |= (sector_buffer[i + 86] & 0x02) << 1;
         value |= (sector_buffer[i + 172] & 0x01) << 5;
         value |= (sector_buffer[i + 172] & 0x02) << 3;
         data_block_1[i] = byte_translation[value ^ prev_value];
         prev_value = value;
      }  

      /*
       * Step 9: write the last 256 disk bytes of the data block, which
       * encodes the top six bits of each sector byte.  Again, each value
       * is exclusive ORed with the previous value to create a running
       * checksum (the first value is exclusive ORed with the last value of
       * the previous step).
       */

      let mut data_block_2 = [0, ..256];
      for i in range(0,256)
      {
         value = (sector_buffer[i] >> 2);
         data_block_2[i] = byte_translation[value ^ prev_value];
         prev_value = value;
      }

      /*
       * Step 10: write the last value as the checksum.
       */
      let checksum = byte_translation[value];

      /*
       * Step 11: write the 3-byte data trailer.
       */
       
      // concat all the arrays
      let result = [
         ~[0xff, ..14],
         ~[0xd5, 0xaa, 0x96],
         address_block.to_owned(),
         ~[0xde, 0xaa, 0xeb],
         ~[0xff, ..6],
         ~[0xd5, 0xaa, 0xad],
         data_block_1.to_owned(),
         data_block_2.to_owned(),
         ~[checksum],
         ~[0xde, 0xaa, 0xeb]
      ].concat_vec();
      assert!(result.len() == RAW_SECTOR_SIZE);
      result //.slice(0, RAW_SECTOR_SIZE).to_owned()
   }

   fn nibblizeTrack(vol:u8, trk:u8, disk:DiskImage) -> RawTrackData
   {
      use std::vec;
      let arr = vec::from_fn(16, |sector| {
         let startindex = skewing_table[sector] as uint << 8;
         return nibblizeSector(vol, trk, sector as u8, disk[trk].slice(startindex, startindex+256));
      }).concat_vec();
      debug!("track {} converted to {:x} raw bytes", trk, arr.len());
      assert!(arr.len() == RAW_SECTOR_SIZE*16);
      let mut fixarr: RawTrackData = [0xff_u8, ..RAW_TRACK_SIZE];
      for i in range(0, arr.len())
      {
         fixarr[i] = arr[i];
      }
      return fixarr;
   }

static PROM: [u8,..256] = [
      0xA2,0x20,0xA0,0x00,0xA2,0x03,0x86,0x3C,0x8A,0x0A,0x24,0x3C,0xF0,0x10,0x05,0x3C
      ,0x49,0xFF,0x29,0x7E,0xB0,0x08,0x4A,0xD0,0xFB,0x98,0x9D,0x56,0x03,0xC8,0xE8,0x10
      ,0xE5,0x20,0x58,0xFF,0xBA,0xBD,0x00,0x01,0x0A,0x0A,0x0A,0x0A,0x85,0x2B,0xAA,0xBD
      ,0x8E,0xC0,0xBD,0x8C,0xC0,0xBD,0x8A,0xC0,0xBD,0x89,0xC0,0xA0,0x50,0xBD,0x80,0xC0
      ,0x98,0x29,0x03,0x0A,0x05,0x2B,0xAA,0xBD,0x81,0xC0,0xA9,0x56,
/*0x20,0xA8,0xFC,*/0xa9,0x00,0xea,
      0x88
      ,0x10,0xEB,0x85,0x26,0x85,0x3D,0x85,0x41,0xA9,0x08,0x85,0x27,0x18,0x08,0xBD,0x8C
      ,0xC0,0x10,0xFB,0x49,0xD5,0xD0,0xF7,0xBD,0x8C,0xC0,0x10,0xFB,0xC9,0xAA,0xD0,0xF3
      ,0xEA,0xBD,0x8C,0xC0,0x10,0xFB,0xC9,0x96,0xF0,0x09,0x28,0x90,0xDF,0x49,0xAD,0xF0
      ,0x25,0xD0,0xD9,0xA0,0x03,0x85,0x40,0xBD,0x8C,0xC0,0x10,0xFB,0x2A,0x85,0x3C,0xBD
      ,0x8C,0xC0,0x10,0xFB,0x25,0x3C,0x88,0xD0,0xEC,0x28,0xC5,0x3D,0xD0,0xBE,0xA5,0x40
      ,0xC5,0x41,0xD0,0xB8,0xB0,0xB7,0xA0,0x56,0x84,0x3C,0xBC,0x8C,0xC0,0x10,0xFB,0x59
      ,0xD6,0x02,0xA4,0x3C,0x88,0x99,0x00,0x03,0xD0,0xEE,0x84,0x3C,0xBC,0x8C,0xC0,0x10
      ,0xFB,0x59,0xD6,0x02,0xA4,0x3C,0x91,0x26,0xC8,0xD0,0xEF,0xBC,0x8C,0xC0,0x10,0xFB
      ,0x59,0xD6,0x02,0xD0,0x87,0xA0,0x00,0xA2,0x56,0xCA,0x30,0xFB,0xB1,0x26,0x5E,0x00
      ,0x03,0x2A,0x5E,0x00,0x03,0x2A,0x91,0x26,0xC8,0xD0,0xEE,0xE6,0x27,0xE6,0x3D,0xA5
      ,0x3D,0xCD,0x00,0x08,0xA6,0x2B,0x90,0xDB,0x4C,0x01,0x08,0x00,0x00,0x00,0x00,0x00
   ];

//static phaseup: [int,..4] = [ 3, 5, 7, 1 ];
//static phasedn: [int,..4] = [ 7, 1, 3, 5 ];

   /*
    * Normal byte (lower six bits only) -> disk byte translation table.
    */
   static byte_translation: [u8, ..64] = [
      0x96, 0x97, 0x9a, 0x9b, 0x9d, 0x9e, 0x9f, 0xa6,
      0xa7, 0xab, 0xac, 0xad, 0xae, 0xaf, 0xb2, 0xb3,
      0xb4, 0xb5, 0xb6, 0xb7, 0xb9, 0xba, 0xbb, 0xbc,
      0xbd, 0xbe, 0xbf, 0xcb, 0xcd, 0xce, 0xcf, 0xd3,
      0xd6, 0xd7, 0xd9, 0xda, 0xdb, 0xdc, 0xdd, 0xde,
      0xdf, 0xe5, 0xe6, 0xe7, 0xe9, 0xea, 0xeb, 0xec,
      0xed, 0xee, 0xef, 0xf2, 0xf3, 0xf4, 0xf5, 0xf6,
      0xf7, 0xf9, 0xfa, 0xfb, 0xfc, 0xfd, 0xfe, 0xff
   ];

   /*
    * Sector skewing table.
    */
   static skewing_table: [u8, ..16] = [
      0,7,14,6,13,5,12,4,11,3,10,2,9,1,8,15
   ];

// TESTS

#[test]
fn test_nibbilize()
{
   let disk: DiskImage = [[0, ..SECTORS_PER_TRACK*SECTOR_SIZE], ..NUM_TRACKS];
   let track = nibblizeTrack(254, 0, disk);
   //for i in range(0,track.len()) { print!("{:2x} ", track[i]); }
}
