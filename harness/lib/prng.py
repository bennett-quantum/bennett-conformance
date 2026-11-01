"""SplitMix64: a deterministic stream fixed here, independent of the Python version."""
MASK = (1 << 64) - 1


class Prng:
    def __init__(self, seed):
        self.state = seed & MASK

    def u64(self):
        self.state = (self.state + 0x9E3779B97F4A7C15) & MASK
        z = self.state
        z = ((z ^ (z >> 30)) * 0xBF58476D1CE4E5B9) & MASK
        z = ((z ^ (z >> 27)) * 0x94D049BB133111EB) & MASK
        return z ^ (z >> 31)

    def below(self, n):
        if n <= 0:
            raise ValueError("below(%d)" % n)
        return self.u64() % n

    def chance(self, numerator, denominator=100):
        return self.below(denominator) < numerator

    def pick(self, seq):
        return seq[self.below(len(seq))]

    def int_of_width(self, width):
        kind = self.below(6)
        top = (1 << width) - 1
        if kind == 0:
            return 0
        if kind == 1:
            return 1 & top
        if kind == 2:
            return top
        if kind == 3:
            return 1 << (width - 1)
        v = 0
        for _ in range((width + 63) // 64):
            v = (v << 64) | self.u64()
        return v & top

    def hex_bytes(self, n):
        return "".join("%02x" % self.below(256) for _ in range(n))
