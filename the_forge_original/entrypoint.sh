#!/bin/bash
set -e

COMMAND=${1:-all}

case $COMMAND in
    cartographer)
        echo "=== Running Cartographer ==="
        python3 /forge/cartographer/extract_drivers.py
        ;;
    
    architect)
        echo "=== Running Architect ==="
        python3 /forge/architect/generate_machines.py
        ;;
    
    foundry)
        echo "=== Running Foundry ==="
        /forge/foundry/build_kernel.sh
        ;;
    
    crucible)
        echo "=== Running Crucible ==="
        /forge/crucible/boot_test.sh
        ;;
    
    all)
        echo "=== The Forge: Full Pipeline ==="
        echo ""
        echo "Step 1/4: Cartographer (extracting driver manifest)"
        python3 /forge/cartographer/extract_drivers.py
        echo ""
        echo "Step 2/4: Architect (generating machine config)"
        python3 /forge/architect/generate_machines.py
        echo ""
        echo "Step 3/4: Foundry (compiling kernel)"
        /forge/foundry/build_kernel.sh
        echo ""
        echo "Step 4/4: Crucible (boot testing)"
        /forge/crucible/boot_test.sh
        echo ""
        echo "=== Pipeline Complete ==="
        ;;
    
    shell)
        exec /bin/bash
        ;;
    
    *)
        echo "Unknown command: $COMMAND"
        echo "Usage: entrypoint.sh [cartographer|architect|foundry|crucible|all|shell]"
        exit 1
        ;;
esac
