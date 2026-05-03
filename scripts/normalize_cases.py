import os
import glob
import xml.etree.ElementTree as ET

def main():
    cases_dir = "tests/cases"
    xml_files = glob.glob(os.path.join(cases_dir, "*.xml"))
    
    count = 0
    for filepath in xml_files:
        try:
            tree = ET.parse(filepath)
            # ET.indent was added in Python 3.9
            if hasattr(ET, "indent"):
                ET.indent(tree, space="    ")
            else:
                print("ET.indent not available in this Python version. Skipping indentation.")
                break
            
            with open(filepath, "wb") as f:
                f.write(b"<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n")
                tree.write(f, encoding="utf-8", xml_declaration=False)
            count += 1
        except Exception as e:
            print(f"Error processing {filepath}: {e}")
            
    print(f"Normalized {count} case files")

if __name__ == "__main__":
    main()
