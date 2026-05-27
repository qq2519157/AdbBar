import struct, zlib, math

W, H = 44, 44
pixels = [(0,0,0,0)] * (W * H)

def set_px(x, y, a):
    if 0 <= x < W and 0 <= y < H:
        pixels[y * W + x] = (255, 255, 255, min(255, max(0, int(a))))

# Phone body outline (white, template-style)
for y in range(8, 38):
    for x in range(16, 29):
        # Outer rounded rect
        corners = [(19,11), (25,11), (19,35), (25,35)]
        skip = False
        for cx, cy in corners:
            if (x < 19 or x > 25) and (y < 11 or y > 35):
                if math.hypot(x-cx, y-cy) > 4:
                    skip = True
                    break
        if skip:
            continue
        # Only draw outline (not filled)
        is_edge = (x <= 17 or x >= 28 or y <= 9 or y >= 37)
        if is_edge:
            edge = min(x-16, 28-x, y-8, 37-y)
            a = min(255, edge * 255)
            set_px(x, y, a)

# Screen line (top speaker)
for x in range(19, 26):
    set_px(x, 12, 180)

# Home button
for y in range(H):
    for x in range(W):
        d = math.hypot(x-22, y-35)
        if d < 2.2:
            set_px(x, y, (2.2-d)/1.0 * 200)

# Signal arcs (3 concentric arcs, upper right)
for y in range(H):
    for x in range(W):
        if x < 30 or y > 16: continue
        d = math.hypot(x-32, y-14)
        for i, r in enumerate([4, 7, 10]):
            if abs(d - r) < 1.0:
                a = (1.0 - abs(d - r)) * (220 - i * 40)
                set_px(x, y, a)

# Build PNG
raw = b''
for row in [pixels[i*W:(i+1)*W] for i in range(H)]:
    raw += b'\x00'
    for r, g, b, a in row:
        raw += bytes([r, g, b, a])

def chunk(t, d):
    c = t + d
    return struct.pack('>I', len(d)) + c + struct.pack('>I', zlib.crc32(c) & 0xffffffff)

ihdr = struct.pack('>IIBBBBB', W, H, 8, 6, 0, 0, 0)
with open('icon.png', 'wb') as f:
    f.write(b'\x89PNG\r\n\x1a\n')
    f.write(chunk(b'IHDR', ihdr))
    f.write(chunk(b'IDAT', zlib.compress(raw, 9)))
    f.write(chunk(b'IEND', b''))
print('Done')
