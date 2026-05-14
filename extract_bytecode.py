#!/usr/bin/env python3
import json
import sys
import re

# Extract deployedBytecode.object from SafeLender.json
with open('out/SafeLender.sol/SafeLender.json', 'r') as f:
    data = json.load(f)

deployed_bytecode = data['deployedBytecode']['object']
# Remove leading "0x"
deployed_bytecode = deployed_bytecode[2:]
print(deployed_bytecode)