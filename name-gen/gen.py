import random

def randLine(filename: str):
    with open(filename) as f:
        lines = f.readlines()
        return lines[random.randint(0, len(lines) -1)].strip()

print(randLine("./adjectives.txt"), "-", randLine("./nouns.txt"), sep="")
