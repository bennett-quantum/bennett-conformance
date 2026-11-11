"""Prove scalar constant placement from named symbols or anonymous ELF images."""
import hashlib
import re

from gate_support import GateError


def require(condition, message):
    if not condition:
        raise GateError('native comparison global layout mismatch: '+message)


def sections(text):
    pattern = (r'^\s*\[\s*\d+\]\s+(\S+)\s+PROGBITS\s+([0-9a-fA-F]+)\s+'
               r'([0-9a-fA-F]+)\s+([0-9a-fA-F]+)\s+([0-9a-fA-F]+)\s+(\S+)\s+')
    return [{'name':m[1], 'address':int(m[2],16), 'offset':int(m[3],16),
             'size':int(m[4],16), 'entry_size':int(m[5],16), 'flags':m[6]}
            for m in re.finditer(pattern,text,re.M)]


def loads(text):
    result = []
    for line in text.splitlines():
        fields = line.split()
        if fields and fields[0] == 'LOAD':
            require(len(fields) >= 8, 'malformed LOAD header')
            result.append({'offset':int(fields[1],16), 'address':int(fields[2],16),
                'file_size':int(fields[4],16), 'memory_size':int(fields[5],16),
                'flags':''.join(fields[6:-1])})
    return result


def images(text):
    section = re.search(r'^section data\n(.*?)(?=^section |\Z)',text,re.M|re.S)
    require(section is not None, 'missing device data images')
    result = {}
    for line in section[1].splitlines():
        if not line.strip(): continue
        fields = line.split()
        require(len(fields) == 2 and re.fullmatch(r'(?:[0-9a-f]{2})+',fields[1]), 'malformed device image')
        require(fields[0] not in result, 'duplicate device image')
        result[fields[0]] = bytes.fromhex(fields[1])
    return result


def anonymous_image(blob, base, size, image, addresses, section_rows, load_rows):
    require(type(base) is int and type(size) is int and size > 0 and len(image) == size,
            'device image size/type mismatch')
    candidates = [s for s in section_rows if s['address'] <= base and base+size <= s['address']+s['size']]
    require(len(candidates) == 1, 'global is not contained in one file-backed section')
    section = candidates[0]; flags = set(section['flags'])
    require({'A','M'} <= flags and not flags & {'W','X'}, 'anonymous global must be allocated read-only merged constant data')
    require(not any(section['address'] <= a < section['address']+section['size'] for a in addresses),
            'named read-only globals must retain their exact symbol addresses')
    entry = section['entry_size']
    require(entry > 0 and (base-section['address']) % entry == 0 and size % entry == 0,
            'global does not span complete merged constant entries')
    overlapping = [s for s in load_rows if s['address'] < base+size and base < s['address']+s['memory_size']]
    require(len(overlapping) == 1, 'global has no unique load mapping')
    load = overlapping[0]
    require('R' in load['flags'] and 'W' not in load['flags'] and base >= load['address']
            and base+size <= load['address']+load['file_size'], 'global is not file-backed read-only load data')
    offset = section['offset']+base-section['address']
    require(offset == load['offset']+base-load['address'], 'section and loader offsets disagree')
    require(0 <= offset and offset+size <= len(blob) and blob[offset:offset+size] == image,
            'ELF bytes differ from the device image')
    return {'method':'anonymous-merged-image', 'base':base, 'size':size, 'section':section['name'],
            'section_address':section['address'], 'file_offset':offset,
            'image_sha256':hashlib.sha256(image).hexdigest()}


def check_globals(binary, layout, object_text, symbols, readelf):
    entries = [f for line in symbols.splitlines() if len(f := line.split()) == 3]
    addresses = {int(f[0],16) for f in entries if f[1].lower() == 'r'}
    named_addresses = {int(f[0],16) for f in entries}
    layout_globals = layout['globals']
    proof = {name:{'method':'named-readonly-symbol', 'base':g['base']}
             for name,g in layout_globals.items() if g['base'] in addresses}
    missing = [(name,g) for name,g in layout_globals.items() if name not in proof]
    if missing:
        section_rows,load_rows = sections(readelf('-S')),loads(readelf('-l'))
        data = images(object_text);blob = binary.read_bytes()
        for name,g in missing:
            require(g.get('image') in data, 'global names a missing device image')
            proof[name] = anonymous_image(blob,g['base'],g.get('size'),data[g['image']],
                                         named_addresses,section_rows,load_rows)
    return {'globals':proof, 'elf_sha256':hashlib.sha256(binary.read_bytes()).hexdigest()}
