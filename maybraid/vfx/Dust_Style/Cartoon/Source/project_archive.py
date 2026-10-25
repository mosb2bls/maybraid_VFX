"""Remove workstation globals from Houdini's standard ASCII-CPIO HIP archive.
Node data and native definitions are unchanged. HIP/HIPFILE are supplied by the
loader; JOB and POSE use relocatable environment-variable references.
"""
from pathlib import Path
def make_relative(path):
    path=Path(path);data=path.read_bytes();offset=0;out=bytearray()
    while data[offset:offset+6]==b'070707':
        header=bytearray(data[offset:offset+76]);n=int(header[59:65],8);size=int(header[65:76],8);name=data[offset+76:offset+76+n];payload=data[offset+76+n:offset+76+n+size];offset+=76+n+size
        if name.rstrip(b'\0')==b'.variables':
            lines=[]
            for line in payload.decode().splitlines():
                if line.startswith(('set -g HIP =','set -g HIPFILE =')):continue
                if line.startswith('set -g JOB ='):line="set -g JOB = '$HIP/..'"
                if line.startswith('set -g POSE ='):line="set -g POSE = '$HOUDINI_USER_PREF_DIR/poselib'"
                lines.append(line)
            payload=('\n'.join(lines)+'\n').encode();header[65:76]=f'{len(payload):011o}'.encode()
        out.extend(header);out.extend(name);out.extend(payload)
        if name.rstrip(b'\0')==b'TRAILER!!!':break
    out.extend(data[offset:]);path.write_bytes(out)
