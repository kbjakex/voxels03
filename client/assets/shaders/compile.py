import subprocess
import os

# Invokes the following for each shader file in the directory:
# glslc filename -o bin/debug_filename.spv
# glslc -O filename -o bin/filename.spv

def check_result(filename: str, result: subprocess.CompletedProcess):
    if result.returncode != 0:
        print(f"Failed to compile shader {filename}")
        if len(result.stdout) != 0:
            print(">> Stdout:")
            print(result.stdout)
            print()
        if len(result.stderr) != 0:
            print(">> Stderr:")
            print(result.stderr)
            print()
        print()

def compile_shader(filename: str):
    check_result(filename, subprocess.run(["glslc", filename, "-o", f"bin/debug_{filename}.spv"], capture_output=True, text=True))
    subprocess.run(["glslc", "-O", filename, "-o", f"bin/{filename}.spv"], capture_output=True)

def main():
    cwd = os.getcwd()
    if not "shaders" in cwd:
        print("compile.py must be ran from within the shaders folder")
        print("You're at:", cwd)
        exit(1)

    for entry in os.listdir():
        if not entry.endswith(".vert") and not entry.endswith(".frag"):
            continue

        compile_shader(entry)

if __name__ == "__main__":
    main()