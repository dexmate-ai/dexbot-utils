#!/usr/bin/env python3
"""Relocated installed C++ consumer and model parity checks. No hardware/network."""
import json
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile

repo = Path(__file__).resolve().parents[1]
sdk = Path(sys.argv[1]).resolve()
with tempfile.TemporaryDirectory(prefix="dexbot-consumer-") as temporary:
    root = Path(temporary)
    relocated = root / "sdk with spaces"
    shutil.copytree(sdk, relocated)
    source = root / "example"
    shutil.copytree(relocated / "share/dexbot/examples/cpp", source)
    shutil.copy2(repo / "bindings/cpp/tests/model.cpp", source / "model_tests.cpp")
    (source / "custom.yaml").write_text("schema_version: 1\nrobot:\n  model: test\ncomponents: {}\n")
    with (source / "CMakeLists.txt").open("a") as cmake:
        cmake.write("\nadd_executable(model_tests model_tests.cpp)\ntarget_link_libraries(model_tests PRIVATE dexbot::model)\nadd_test(NAME model_tests COMMAND model_tests ${CMAKE_CURRENT_SOURCE_DIR}/custom.yaml)\n")
    build = root / "build"
    subprocess.run(["cmake", "-S", str(source), "-B", str(build), "-DCMAKE_PREFIX_PATH=" + str(relocated)], check=True)
    subprocess.run(["cmake", "--build", str(build), "--parallel", "2"], check=True)
    subprocess.run(["ctest", "--test-dir", str(build), "--output-on-failure"], check=True)
    lines = subprocess.check_output([str(build / "inspect_model"), "--json"], text=True).splitlines()
    for line in lines:
        cpp = json.loads(line)
        rust = json.loads(subprocess.check_output([str(relocated / "bin/dexbot"), "show", cpp["profile_name"]], text=True))
        assert cpp == rust, cpp["profile_name"] + " differs between Rust and C++"
    assert len(lines) == 9, "Missing built-in profiles"
    print("All profiles match between installed C++ SDK and Rust CLI")
