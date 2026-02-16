#!/usr/bin/env python3
"""Auto-generate draw.io architecture diagram from source code."""

import re
from pathlib import Path

SRC_DIR = Path("src")
LIB_FILE = SRC_DIR / "lib.rs"

EXTERNALS = [
    {"id": "libsql", "label": "libsql", "x": 780, "y": 120, "w": 120, "h": 35},
    {"id": "tokio", "label": "Tokio", "x": 780, "y": 165, "w": 120, "h": 35},
    {"id": "rayon", "label": "Rayon", "x": 780, "y": 210, "w": 120, "h": 35},
    {"id": "rand", "label": "Rand", "x": 780, "y": 255, "w": 120, "h": 35},
]

KNOWN_TYPES = {
    "error": "Error",
    "hyperdim": "HVec10240",
    "reservoir": "Reservoir",
    "singularity": "Concept",
    "persistence": "Persistence",
    "framework": "Framework",
    "wasm": "WASM",
}

FLOW_DEPS = {
    "framework": ["reservoir", "singularity", "persistence", "hyperdim"],
    "reservoir": ["hyperdim"],
    "singularity": ["hyperdim"],
    "persistence": [],
    "hyperdim": [],
}

STYLES = {
    "orchestrator": ("#dae8fc", "#6c8ebf"),
    "core": ("#d5e8d4", "#82b366"),
    "wasm": ("#fff3e0", "#ff9800"),
    "error": ("#f8cecc", "#b85450"),
    "external": ("#e1d5c9", "#666666"),
}

SKIP_MODULES = {"prelude"}

def parse_lib_rs():
    content = LIB_FILE.read_text()
    
    modules = []
    for line in content.split("\n"):
        line = line.strip()
        if line.startswith("pub mod "):
            mod_name = line.split()[2].rstrip(";")
            modules.append(mod_name)
    return modules

def parse_module(module_name):
    file_path = SRC_DIR / f"{module_name}.rs"
    if not file_path.exists():
        return {"structs": [], "is_wasm_module": False}
    
    content = file_path.read_text()
    
    is_wasm_module = "wasm" in module_name
    
    structs = []
    for line in content.split("\n"):
        line = line.strip()
        if line.startswith("pub struct "):
            name = line.split()[2].split("<")[0].split("(")[0]
            structs.append(name)
        elif line.startswith("pub enum "):
            name = line.split()[2]
            structs.append(name)
    
    return {"structs": structs, "is_wasm_module": is_wasm_module}

def auto_discover():
    modules = parse_lib_rs()
    
    discovered = []
    y_core = 150
    
    order = ["error", "hyperdim", "reservoir", "singularity", "persistence", "framework", "wasm"]
    
    for mod in modules:
        if mod in SKIP_MODULES:
            continue
        
        info = parse_module(mod)
        
        if mod == "error":
            style = "error"
            label = "Error"
            x, y = 80, 60
            w, h = 100, 40
        elif info["is_wasm_module"]:
            style = "wasm"
            label = "WASM Stub\n(persistence_wasm)" if mod == "persistence_wasm" else "WASM\nBindings"
            x, y = 380, 510 if mod == "persistence_wasm" else 450
            w, h = 120, 50
        elif mod == "framework":
            style = "orchestrator"
            label = "Framework\n(Orchestrator)"
            x, y = 380, 200
            w, h = 180, 60
        else:
            style = "core"
            known = KNOWN_TYPES.get(mod, mod.capitalize())
            label = f"{mod.capitalize()}\n({known})"
            x, y = 80, y_core
            w, h = 140, 55
            y_core += 65
        
        discovered.append({
            "id": mod,
            "label": label,
            "x": x, "y": y, "w": w, "h": h,
            "style": style,
            "structs": info["structs"],
        })
    
    discovered.sort(key=lambda m: order.index(m["id"]) if m["id"] in order else 999)
    
    return discovered

def auto_flows(modules):
    flows = []
    for mod in modules:
        deps = FLOW_DEPS.get(mod["id"], [])
        for dep in deps:
            if any(m["id"] == dep for m in modules):
                flows.append((mod["id"], dep))
    return flows

def generate_xml(modules):
    cells = []
    cell_id = 0
    
    def add_cell(value, x, y, w, h, style):
        nonlocal cell_id
        cid = f"c{cell_id}"
        cell_id += 1
        cells.append(f'''        <mxCell id="{cid}" value="{value}" style="{style}" vertex="1" parent="1">
          <mxGeometry x="{x}" y="{y}" width="{w}" height="{h}" as="geometry" />
        </mxCell>''')
        return cid
    
    ids = {}
    
    for m in modules:
        fill, stroke = STYLES[m["style"]]
        style = f"rounded=1;whiteSpace=wrap;html=1;fillColor={fill};strokeColor={stroke}"
        if m["style"] == "wasm":
            style += ";dashed=1"
        ids[m["id"]] = add_cell(m["label"], m["x"], m["y"], m["w"], m["h"], style)
    
    for e in EXTERNALS:
        fill, stroke = STYLES["external"]
        style = f"rounded=0;whiteSpace=wrap;html=1;fillColor={fill};strokeColor={stroke};dashed=1"
        ids[e["id"]] = add_cell(e["label"], e["x"], e["y"], e["w"], e["h"], style)
    
    flows = auto_flows(modules)
    for src, dst in flows:
        cid = f"e{cell_id}"
        cell_id += 1
        cells.append(f'''        <mxCell id="{cid}" style="edgeStyle=orthogonalEdgeStyle;rounded=0;html=1;strokeWidth=2;strokeColor=#666666" edge="1" parent="1" source="{ids[src]}" target="{ids[dst]}">
          <mxGeometry relative="1" as="geometry" />
        </mxCell>''')
    
    return f'''<mxfile host="app.diagrams.net">
  <diagram name="Architecture">
    <mxGraphModel dx="1200" dy="800" grid="1" gridSize="10" guides="1" tooltips="1" connect="1" arrows="1" fold="1" page="1" pageScale="1" pageWidth="1000" pageHeight="600" math="0" shadow="0">
      <root>
        <mxCell id="0" />
        <mxCell id="1" parent="0" />
{chr(10).join(cells)}
      </root>
    </mxGraphModel>
  </diagram>
</mxfile>'''

def main():
    print("Parsing src/lib.rs and modules...")
    modules = auto_discover()
    print(f"Discovered {len(modules)} modules:")
    for m in modules:
        print(f"  - {m['id']}: {m['style']} ({len(m['structs'])} types)")
    
    xml = generate_xml(modules)
    
    out_path = Path("docs/architecture/arch.drawio")
    out_path.write_text(xml)
    print(f"\nGenerated: {out_path} ({len(xml.split(chr(10)))} lines)")

if __name__ == "__main__":
    main()
