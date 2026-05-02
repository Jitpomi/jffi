#!/usr/bin/env python3
import sys
from app import {{name_pascal}}Application

def main():
    app = {{name_pascal}}Application()
    return app.run(sys.argv)

if __name__ == '__main__':
    sys.exit(main())
