
import time
import os
import sys
import logging
import statistics
from pathlib import Path

# Add backend to path to import python module
import sys
import os
from pathlib import Path

# Add the project root directory to sys.path
project_root = Path(__file__).resolve().parent.parent.parent
sys.path.append(str(project_root))

# Load PyXmlParser directly from file to avoid triggering backend.parsers.__init__
# which causes circular/missing import issues with config
import importlib.util
xml_parser_path = project_root / 'backend' / 'parsers' / 'xml_parser.py'
spec = importlib.util.spec_from_file_location("xml_parser", xml_parser_path)
xml_parser_mod = importlib.util.module_from_spec(spec)
sys.modules["xml_parser"] = xml_parser_mod
spec.loader.exec_module(xml_parser_mod)
PyXmlParser = xml_parser_mod.XmlParser
try:
    from nwn2_rust import XmlParser as RustXmlParser
except ImportError:
    print("Rust Extension not found!")
    sys.exit(1)

# Configuration
TEST_FILE = Path(os.path.dirname(__file__)) / "../sample_save/000000 - 23-07-2025-13-06/globals.xml"
ITERATIONS = 50

def benchmark_function(name, func, *args):
    times = []
    for _ in range(ITERATIONS):
        start = time.perf_counter()
        func(*args)
        end = time.perf_counter()
        times.append((end - start) * 1000) # ms
    
    avg = statistics.mean(times)
    median = statistics.median(times)
    stdev = statistics.stdev(times) if len(times) > 1 else 0
    return avg, median, stdev

def run_benchmarks():
    if not TEST_FILE.exists():
        print(f"Test file not found: {TEST_FILE}")
        # Create a large dummy file
        print("Generating large dummy XML...")
        content = ['<?xml version="1.0" ?>\n<Globals>\n<Integers>']
        for i in range(10000):
            content.append(f'<Integer><Name>Var_{i}</Name><Value>{i}</Value></Integer>')
            content.append(f'<Integer><Name>00_nInfluencecompanion_{i}</Name><Value>{i}</Value></Integer>')
            content.append(f'<Integer><Name>quest_{i}_Done</Name><Value>1</Value></Integer>')
        content.append('</Integers>\n<Strings>')
        for i in range(5000):
            content.append(f'<String><Name>Str_{i}</Name><Value>Unknown Value {i}</Value></String>')
        content.append('</Strings>\n<Floats>')
        for i in range(5000):
            content.append(f'<Float><Name>Flt_{i}</Name><Value>{i}.5</Value></Float>')
        content.append('</Floats>\n<Vectors />\n</Globals>')
        xml_content = "".join(content)
    else:
        print(f"Using test file: {TEST_FILE}")
        with open(TEST_FILE, 'r', encoding='utf-8') as f:
            xml_content = f.read()

    print(f"XML Size: {len(xml_content)/1024:.2f} KB")
    print("-" * 60)
    print(f"{'Operation':<30} | {'Python (ms)':<10} | {'Rust (ms)':<10} | {'Speedup':<8}")
    print("-" * 60)

    # 1. Parsing Benchmark
    py_parse_avg, _, _ = benchmark_function("Python Parse", PyXmlParser, xml_content)
    rust_parse_avg, _, _ = benchmark_function("Rust Parse", RustXmlParser, xml_content)
    print(f"{'Parse':<30} | {py_parse_avg:<10.2f} | {rust_parse_avg:<10.2f} | {py_parse_avg/rust_parse_avg:.1f}x")

    # Instantiate parsers for method benchmarks
    py_parser = PyXmlParser(xml_content)
    rust_parser = RustXmlParser(xml_content)

    # 2. Get Companion Status
    py_comp_avg, _, _ = benchmark_function("Py Companion", py_parser.get_companion_status)
    rust_comp_avg, _, _ = benchmark_function("Rust Companion", rust_parser.get_companion_status)
    print(f"{'Get Companion Status':<30} | {py_comp_avg:<10.3f} | {rust_comp_avg:<10.3f} | {py_comp_avg/rust_comp_avg:.1f}x")

    # 3. Get Quest Overview
    py_quest_avg, _, _ = benchmark_function("Py Quest", py_parser.get_quest_overview)
    if hasattr(rust_parser, 'get_quest_overview'):
        rust_quest_avg, _, _ = benchmark_function("Rust Quest", rust_parser.get_quest_overview)
        print(f"{'Get Quest Overview':<30} | {py_quest_avg:<10.3f} | {rust_quest_avg:<10.3f} | {py_quest_avg/rust_quest_avg:.1f}x")

    # 4. Get Full Summary
    py_full_avg, _, _ = benchmark_function("Py Full Summary", py_parser.get_full_summary)
    rust_full_avg, _, _ = benchmark_function("Rust Full Summary", rust_parser.get_full_summary)
    print(f"{'Get Full Summary':<30} | {py_full_avg:<10.3f} | {rust_full_avg:<10.3f} | {py_full_avg/rust_full_avg:.1f}x")
    
    # 5. Serialization
    py_ser_avg, _, _ = benchmark_function("Py Serialize", py_parser.to_xml_string)
    rust_ser_avg, _, _ = benchmark_function("Rust Serialize", rust_parser.to_xml_string)
    print(f"{'Serialize':<30} | {py_ser_avg:<10.2f} | {rust_ser_avg:<10.2f} | {py_ser_avg/rust_ser_avg:.1f}x")

    print("-" * 60)

if __name__ == "__main__":
    run_benchmarks()
