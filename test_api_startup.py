#!/usr/bin/env python3
"""
Test script to verify the cloud API gateway can start successfully.
This will test if the compilation issues have been resolved.
"""

import subprocess
import sys
import os
import time
import signal
from pathlib import Path

def test_api_startup():
    """Test if the API gateway can start successfully."""
    print("Testing LogNarrator Cloud API Gateway Startup")
    print(f"Python version: {sys.version}")
    print(f"Current directory: {os.getcwd()}")
    
    # Change to cloud API directory
    api_dir = Path("src/cloud")
    if not api_dir.exists():
        print(f"Error: API directory {api_dir} does not exist")
        return False
    
    os.chdir(api_dir)
    print(f"Changed to directory: {os.getcwd()}")
    
    # Check if main.py exists
    if not Path("api/main.py").exists():
        print("Error: api/main.py not found")
        return False
    
    print("\n" + "="*60)
    print("STARTING API GATEWAY TEST")
    print("="*60)
    
    # Start the API server in a subprocess
    try:
        print("Starting API gateway...")
        process = subprocess.Popen(
            [sys.executable, "-m", "api.main"],
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            preexec_fn=os.setsid  # Create new process group
        )
        
        # Wait a few seconds for startup
        startup_time = 10
        print(f"Waiting {startup_time} seconds for startup...")
        
        for i in range(startup_time):
            if process.poll() is not None:
                # Process has terminated
                stdout, stderr = process.communicate()
                print(f"Process terminated early with return code: {process.returncode}")
                print(f"STDOUT:\n{stdout}")
                print(f"STDERR:\n{stderr}")
                return False
            time.sleep(1)
            print(f"  {i+1}/{startup_time} seconds...")
        
        # Check if process is still running
        if process.poll() is None:
            print("✅ SUCCESS: API gateway started successfully!")
            
            # Try to get some output
            try:
                stdout, stderr = process.communicate(timeout=2)
                if stdout:
                    print(f"STDOUT:\n{stdout}")
                if stderr:
                    print(f"STDERR:\n{stderr}")
            except subprocess.TimeoutExpired:
                # Process is still running, which is good
                print("Process is running and responsive")
            
            # Terminate the process gracefully
            print("Terminating API gateway...")
            try:
                os.killpg(os.getpgid(process.pid), signal.SIGTERM)
                process.wait(timeout=5)
            except subprocess.TimeoutExpired:
                print("Force killing process...")
                os.killpg(os.getpgid(process.pid), signal.SIGKILL)
                process.wait()
            
            print("✅ API gateway stopped successfully")
            return True
        else:
            stdout, stderr = process.communicate()
            print(f"❌ FAILED: Process terminated with return code: {process.returncode}")
            print(f"STDOUT:\n{stdout}")
            print(f"STDERR:\n{stderr}")
            return False
            
    except Exception as e:
        print(f"❌ ERROR: Exception during API startup test: {e}")
        try:
            if 'process' in locals() and process.poll() is None:
                os.killpg(os.getpgid(process.pid), signal.SIGKILL)
        except:
            pass
        return False
    
    finally:
        print("\n" + "="*60)
        print("API STARTUP TEST COMPLETE")
        print("="*60)

def main():
    """Main test function."""
    success = test_api_startup()
    
    if success:
        print("\n🎉 RESULT: The cloud API gateway compilation issues have been RESOLVED!")
        print("The API can start successfully with the current dependency versions.")
    else:
        print("\n❌ RESULT: There are still issues with the API gateway startup.")
        print("Further investigation may be needed.")
    
    return success

if __name__ == "__main__":
    success = main()
    sys.exit(0 if success else 1)