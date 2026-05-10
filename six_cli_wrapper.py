import sys
import os
import subprocess

def main():
    base_dir = os.path.dirname(os.path.abspath(__file__))
    bin_path = os.path.join(base_dir, "target", "release", "six.exe")
    
    if not os.path.exists(bin_path):
        bin_path = os.path.join(base_dir, "target", "debug", "six.exe")
    
    if not os.path.exists(bin_path):
        print("[SixC ERROR] Six binary not found. Please run 'cargo build --release' first.")
        sys.exit(1)
        
    args = [bin_path] + sys.argv[1:]
    subprocess.run(args)

if __name__ == "__main__":
    main()
