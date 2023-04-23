import os

sloc = 0
tloc = 0
files = os.listdir('src')

for file in files:
    with open(f"./src/{file}") as f:
        s, t = 0, 0
        for line in f:
            t += 1
            line = line.strip()
            if line == "" or (len(line) >= 2 and line[0:2] == "//"):
                continue
            s += 1
        sloc += s
        tloc += t
        print(f"{file}: {s}/{t}")

print(f"sloc: {sloc}")
print(f"tloc: {tloc}")

