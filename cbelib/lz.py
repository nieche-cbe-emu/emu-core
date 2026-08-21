
import struct

def decompress(src, out_size):
    out = bytearray()
    i = 0
    n = len(src)
    while i < n and len(out) < out_size:
        c = src[i]
        if c & 0x80:
            k = c & 0x7F
            i += 1
            out += src[i:i + k]
            i += k
        else:
            if i + 1 >= n:
                break
            k = c >> 1
            dist = ((c & 1) << 8) | src[i + 1]
            i += 2
            if dist == 0 or dist > len(out):
                break
            for _ in range(min(k, out_size - len(out))):
                out.append(out[-dist])
    return bytes(out)

def unpack_entry(data):

    if not data or data[0] != 2:
        return None
    comp = struct.unpack_from(">I", data, 1)[0]
    unc = struct.unpack_from(">I", data, 5)[0]
    return decompress(data[9:9 + comp], unc)
