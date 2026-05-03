import os
import xml.etree.ElementTree as ET

def main():
    testsuite_path = "tests/testsuite.xml"
    cases_dir = "tests/cases"

    if not os.path.exists(cases_dir):
        os.makedirs(cases_dir)

    tree = ET.parse(testsuite_path)
    root = tree.getroot()

    manifest_root = ET.Element("testsuite", {"name": "Rawk Integration Tests"})

    for i, testcase in enumerate(root.findall("testcase"), 1):
        name = testcase.get("name")
        filename = f"{i:04d}_{name}.xml"
        filepath = os.path.join(cases_dir, filename)

        # Write testcase XML
        case_tree = ET.ElementTree(testcase)
        with open(filepath, "wb") as f:
            f.write(b"<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n")
            case_tree.write(f, encoding="utf-8", xml_declaration=False)

        # Add to manifest
        ET.SubElement(manifest_root, "case", {"file": filename})

    # Write manifest XML
    manifest_tree = ET.ElementTree(manifest_root)
    with open(testsuite_path, "wb") as f:
        f.write(b"<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n")
        manifest_tree.write(f, encoding="utf-8", xml_declaration=False)

    print(f"Migrated {i} testcases to {cases_dir}/, manifest written to {testsuite_path}")

if __name__ == "__main__":
    main()
