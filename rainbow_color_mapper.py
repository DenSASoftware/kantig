#!/usr/bin/env python3
def value_to_rgb(value): # 0 <= value < 256 * 6
    n, m = divmod(value, 256)
    if b % 2 == 0:
        m = 255 - m

    if n == 0: return (255, m, 0)
    elif n == 1: return (m, 255, 0)
    elif n == 2: return (0, 255, m)
    elif n == 3: return (0, m, 255)
    elif n == 4: return (m, 0, 255)
    else: return (255, 0, m)

def blend(a, b, factor):
    return int(a * factor + b * (1 - factor))

r, g, b = [int(i) for i in input().split()]
x1, y1, x2, y2, x3, y3 = [int(i) for i in input().split()]
width, height = [int(i) for i in input().split()]

center = (
    (x1 + x2 + x3) / 3,
    (y1 + y2 + y3) / 3
)

val = int(center[0] / width * 256 * 6)
(r2, g2, b2) = value_to_rgb(val)

print(
    blend(r, r2, 0.75),
    blend(g, g2, 0.75),
    blend(b, b2, 0.75)
)

