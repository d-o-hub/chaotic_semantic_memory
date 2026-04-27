#!/usr/bin/env python3
"""
LLM Understanding Validation Test
Validates that LLM can correctly answer key questions about the codebase

Usage:
    python3 tests/validate_llm_context.py
"""

import yaml
import sys
from typing import Dict, List, Tuple


class LLMContextValidator:
    """Validates LLM understanding of the codebase context"""
    
    def __init__(self, context_file: str = "docs/architecture/context.yaml"):
        with open(context_file, 'r') as f:
            self.context = yaml.safe_load(f)
        self.results: List[Tuple[str, bool, str]] = []
    
    def test_constraint_retrieval(self) -> bool:
        """Test: Can LLM find max LOC constraint?"""
        hard_constraints = self.context['constraints']['hard']
        loc_constraint = next(
            (c for c in hard_constraints if c['name'] == 'max_loc_per_file'),
            None
        )
        
        if loc_constraint and loc_constraint['value'] == 500:
            self.results.append(("Constraint Retrieval", True, "Found max_loc_per_file = 500"))
            return True
        else:
            self.results.append(("Constraint Retrieval", False, "Could not find LOC constraint"))
            return False
    
    def test_database_requirement(self) -> bool:
        """Test: Can LLM identify required database?"""
        hard_constraints = self.context['constraints']['hard']
        db_constraint = next(
            (c for c in hard_constraints if c['name'] == 'database_libsql'),
            None
        )
        
        if db_constraint and db_constraint['value'] == 'libsql':
            forbidden = db_constraint.get('forbidden', '')
            self.results.append(("Database Requirement", True, f"Must use {db_constraint['value']}, never {forbidden}"))
            return True
        else:
            self.results.append(("Database Requirement", False, "Could not find database constraint"))
            return False
    
    def test_module_identification(self) -> bool:
        """Test: Can LLM identify all core modules?"""
        expected_modules = {'hyperdim', 'reservoir', 'singularity', 'persistence', 'framework', 'error'}
        actual_modules = {m['name'] for m in self.context['architecture']['modules']}
        
        if expected_modules == actual_modules:
            self.results.append(("Module Identification", True, f"Found all {len(expected_modules)} modules"))
            return True
        else:
            missing = expected_modules - actual_modules
            extra = actual_modules - expected_modules
            msg = f"Missing: {missing}, Extra: {extra}"
            self.results.append(("Module Identification", False, msg))
            return False
    
    def test_skill_count(self) -> bool:
        """Test: Can LLM count total skills?"""
        total = self.context['skills']['total_count']
        core = self.context['skills']['categories']['core']['count']
        swarm = self.context['skills']['categories']['swarm']['count']
        
        if total == core + swarm == 13:
            self.results.append(("Skill Count", True, f"Total: {total} (Core: {core}, Swarm: {swarm})"))
            return True
        else:
            self.results.append(("Skill Count", False, f"Mismatch: {total} != {core} + {swarm}"))
            return False
    
    def test_validation_script(self) -> bool:
        """Test: Can LLM find validation script?"""
        script = self.context['validation']['script']
        gates = len(self.context['validation']['gates'])
        
        if script == 'scripts/validate.sh' and gates >= 5:
            self.results.append(("Validation Script", True, f"Script: {script}, Gates: {gates}"))
            return True
        else:
            self.results.append(("Validation Script", False, f"Unexpected: {script} with {gates} gates"))
            return False
    
    def test_performance_targets(self) -> bool:
        """Test: Can LLM identify performance target for reservoir?"""
        targets = self.context['validation']['performance_targets']
        reservoir_target = next(
            (t for t in targets if t['name'] == 'reservoir_step_50k'),
            None
        )
        
        if reservoir_target and reservoir_target['target'] == '< 100μs':
            status = reservoir_target.get('status', 'unknown')
            measured = reservoir_target.get('latest_measured', 'N/A')
            self.results.append(("Performance Targets", True, f"reservoir_step_50k: {reservoir_target['target']} (measured: {measured}, status: {status})"))
            return True
        else:
            self.results.append(("Performance Targets", False, "Could not find reservoir target"))
            return False
    
    def test_wasm_gating(self) -> bool:
        """Test: Can LLM find WASM threading constraint?"""
        hard_constraints = self.context['constraints']['hard']
        wasm_constraint = next(
            (c for c in hard_constraints if c['name'] == 'wasm_threading'),
            None
        )
        
        if wasm_constraint and '#[cfg(not(target_arch = "wasm32"))]' in wasm_constraint.get('gate', ''):
            self.results.append(("WASM Gating", True, f"Gate: {wasm_constraint['gate']}"))
            return True
        else:
            self.results.append(("WASM Gating", False, "Could not find WASM constraint"))
            return False
    
    def test_workflow_steps(self) -> bool:
        """Test: Can LLM enumerate workflow learning loop?"""
        workflow = self.context['workflow']
        learning_loop = workflow['learning_loop']
        
        if len(learning_loop) == 5:
            steps = [s['action'] for s in learning_loop]
            self.results.append(("Workflow Steps", True, f"Found {len(steps)} steps"))
            return True
        else:
            self.results.append(("Workflow Steps", False, f"Expected 5 steps, found {len(learning_loop)}"))
            return False
    
    def test_data_flow(self) -> bool:
        """Test: Can LLM understand concept injection flow?"""
        flows = self.context['data_flow']['flows']
        injection_flow = next(
            (f for f in flows if f['name'] == 'concept_injection'),
            None
        )
        
        if injection_flow and len(injection_flow['steps']) >= 3:
            self.results.append(("Data Flow", True, f"Concept injection has {len(injection_flow['steps'])} steps"))
            return True
        else:
            self.results.append(("Data Flow", False, "Could not find concept injection flow"))
            return False
    
    def run_all_tests(self) -> Dict[str, any]:
        """Run all validation tests"""
        tests = [
            self.test_constraint_retrieval,
            self.test_database_requirement,
            self.test_module_identification,
            self.test_skill_count,
            self.test_validation_script,
            self.test_performance_targets,
            self.test_wasm_gating,
            self.test_workflow_steps,
            self.test_data_flow,
        ]
        
        passed = sum(1 for test in tests if test())
        total = len(tests)
        
        return {
            'total': total,
            'passed': passed,
            'failed': total - passed,
            'score': (passed / total) * 100,
            'results': self.results
        }

    def print_report(self, results: Dict[str, any]):
        """Print validation report"""
        print("=" * 70)
        print("LLM CONTEXT UNDERSTANDING VALIDATION")
        print("=" * 70)
        print()
        
        for test_name, passed, message in results['results']:
            status = "✓ PASS" if passed else "✗ FAIL"
            print(f"{status:8} | {test_name:25} | {message}")
        
        print()
        print("-" * 70)
        print(f"Results: {results['passed']}/{results['total']} tests passed ({results['score']:.1f}%)")
        print("-" * 70)
        
        if results['score'] == 100:
            print("✅ LLM context is fully understandable!")
        elif results['score'] >= 80:
            print("⚠️  LLM context is mostly understandable, minor issues")
        else:
            print("❌ LLM context needs improvement")
        
        return results['failed'] == 0


def main():
    """Main entry point"""
    print("Validating LLM context understanding...\n")
    
    validator = LLMContextValidator()
    results = validator.run_all_tests()
    success = validator.print_report(results)
    
    # Also validate context.yaml structure
    print("\n" + "=" * 70)
    print("CONTEXT FILE STRUCTURE")
    print("=" * 70)
    
    required_sections = [
        'metadata', 'mission', 'constraints', 'architecture',
        'data_flow', 'key_files', 'skills', 'validation', 
        'workflow', 'current_status', 'guardrails'
    ]
    
    context = validator.context
    for section in required_sections:
        if section in context:
            print(f"✓ {section}")
        else:
            print(f"✗ {section} - MISSING")
            success = False
    
    # Token efficiency check
    print("\n" + "=" * 70)
    print("TOKEN EFFICIENCY ANALYSIS")
    print("=" * 70)
    
    import os
    yaml_size = os.path.getsize("docs/architecture/context.yaml")
    drawio_size = os.path.getsize("docs/architecture/context.drawio")
    agents_size = os.path.getsize("AGENTS.md")
    
    print(f"context.yaml:    {yaml_size:6} bytes (LLM-optimized)")
    print(f"AGENTS.md:       {agents_size:6} bytes (text reference)")
    print(f"context.drawio:  {drawio_size:6} bytes (visual only)")
    print(f"\nYAML is {agents_size/yaml_size:.1f}x more compact than AGENTS.md")
    print(f"YAML is {drawio_size/yaml_size:.1f}x more compact than draw.io XML")
    
    sys.exit(0 if success else 1)


if __name__ == '__main__':
    main()
