import struct
import json
import os
import sys

def read_int(f):
    offset = f.tell()
    data = f.read(4)
    if len(data) < 4:
        return None, offset
    return struct.unpack('<I', data)[0], offset

def read_string(f):
    offset = f.tell()
    length_curr, _ = read_int(f)
    if length_curr is None:
        return "", offset
    if length_curr == 0:
        return "", offset
    data = f.read(length_curr)
    try:
        return data.decode('utf-8', errors='replace'), offset
    except:
        return data.hex(), offset

def parse_playerinfo(filepath):
    if not os.path.exists(filepath):
        print(json.dumps({"error": f"File not found: {filepath}"}))
        return

    data = {"FilePath": filepath}
    try:
        with open(filepath, 'rb') as f:
            data['FirstName'], _ = read_string(f)
            data['LastName'], _ = read_string(f)
            data['Subrace'], _ = read_string(f)
            data['Alignment'], _ = read_string(f)
            
            # Unknown Data Block
            unk1_val, unk1_off = read_int(f)
            unk2_val, unk2_off = read_int(f)
            
            data['Unknown1'] = {
                "Value": unk1_val,
                "Hex": f"0x{unk1_val:08x}" if unk1_val is not None else None
            }
            data['Unknown2'] = {
                "Value": unk2_val,
                "Hex": f"0x{unk2_val:08x}" if unk2_val is not None else None
            }

            data['BackgroundFeatID'], _ = read_int(f)
            
            class_count, _ = read_int(f)
            data['ClassCount'] = class_count
            data['Classes'] = []
            
            if class_count is not None:
                for _ in range(class_count):
                    cls = {}
                    cls['Name'], _ = read_string(f)
                    lvl_byte = f.read(1)
                    if lvl_byte:
                        cls['Level'] = lvl_byte[0]
                    else:
                        cls['Level'] = None
                    data['Classes'].append(cls)
            
            data['Deity'], _ = read_string(f)
    except Exception as e:
        data['Error'] = str(e)

    print(json.dumps(data, indent=2))

if __name__ == "__main__":
    paths = [
        r"C:\Users\01tee\Documents\Neverwinter Nights 2\saves\000046 - 23-07-2025-00-55\playerinfo.bin",
        r"C:\Users\01tee\Documents\Neverwinter Nights 2\saves\000049 - 01-08-2025-11-19\playerinfo.bin",
        r"C:\Users\01tee\Documents\Neverwinter Nights 2\saves\000021 - 21-07-2025-17-26\playerinfo.bin",
        r"C:\Users\01tee\Documents\Neverwinter Nights 2\saves\000004 - 20-07-2025-09-57\playerinfo.bin"
    ]
    for p in paths:
        parse_playerinfo(p)
