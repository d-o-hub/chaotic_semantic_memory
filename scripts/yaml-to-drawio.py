#!/usr/bin/env python3
"""
YAML to Draw.io Converter
Converts structured context.yaml to draw.io XML format
"""

import yaml
import sys
import xml.etree.ElementTree as ET
from datetime import datetime, timezone

def escape_xml(text):
    return (text
            .replace('&', '&amp;')
            .replace('<', '&lt;')
            .replace('>', '&gt;')
            .replace('"', '&quot;'))

def yaml_to_drawio(yaml_file, output_file):
    with open(yaml_file, 'r') as f:
        data = yaml.safe_load(f)
    
    mxfile = ET.Element('mxfile', {
        'host': 'app.diagrams.net',
        'modified': datetime.now(timezone.utc).strftime('%Y-%m-%dT%H:%M:%SZ'),
        'agent': 'yaml-to-drawio-converter',
        'version': '24.0.0'
    })
    
    diagram = ET.SubElement(mxfile, 'diagram', {
        'id': 'context-diagram',
        'name': 'LLM Context Overview'
    })
    
    graph_model = ET.SubElement(diagram, 'mxGraphModel', {
        'dx': '1600', 'dy': '1200', 'grid': '1', 'gridSize': '10',
        'guides': '1', 'tooltips': '1', 'connect': '1', 'arrows': '1',
        'fold': '1', 'page': '1', 'pageScale': '1',
        'pageWidth': '1400', 'pageHeight': '1000'
    })
    
    root = ET.SubElement(graph_model, 'root')
    ET.SubElement(root, 'mxCell', {'id': '0'})
    ET.SubElement(root, 'mxCell', {'id': '1', 'parent': '0'})
    
    # Title
    title = f"{data['metadata']['project_name']} - LLM Context"
    title_cell = ET.SubElement(root, 'mxCell', {
        'id': 'title', 'value': escape_xml(title),
        'style': 'text;html=1;strokeColor=none;fillColor=none;align=center;fontSize=16;',
        'vertex': '1', 'parent': '1'
    })
    ET.SubElement(title_cell, 'mxGeometry', {
        'x': '450', 'y': '20', 'width': '500', 'height': '40', 'as': 'geometry'
    })
    
    # Mission
    mission = ET.SubElement(root, 'mxCell', {
        'id': 'mission', 'value': escape_xml(f"Mission: {data['mission']['statement']}"),
        'style': 'rounded=1;whiteSpace=wrap;html=1;fillColor=#d5e8d4;strokeColor=#82b366;',
        'vertex': '1', 'parent': '1'
    })
    ET.SubElement(mission, 'mxGeometry', {
        'x': '40', 'y': '80', 'width': '400', 'height': '60', 'as': 'geometry'
    })
    
    tree = ET.ElementTree(mxfile)
    ET.indent(tree, space='  ')
    tree.write(output_file, encoding='utf-8', xml_declaration=True)
    print(f"Generated {output_file}")

if __name__ == '__main__':
    yaml_to_drawio(sys.argv[1], sys.argv[2])
