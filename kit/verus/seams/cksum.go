package main

// cksumTable is the CRC-32 table of POSIX cksum (polynomial 0x04C11DB7,
// most significant bit first).
var cksumTable = func() [256]uint32 {
	var t [256]uint32
	for i := range t {
		c := uint32(i) << 24
		for range 8 {
			if c&0x80000000 != 0 {
				c = c<<1 ^ 0x04C11DB7
			} else {
				c <<= 1
			}
		}
		t[i] = c
	}
	return t
}()

// cksum is the CRC POSIX cksum prints for data: the CRC of the bytes,
// then of the length's bytes, least significant first, complemented.
func cksum(data string) uint32 {
	var crc uint32
	step := func(b byte) { crc = crc<<8 ^ cksumTable[byte(crc>>24)^b] }
	for i := 0; i < len(data); i++ {
		step(data[i])
	}
	for n := len(data); n > 0; n >>= 8 {
		step(byte(n))
	}
	return ^crc
}
