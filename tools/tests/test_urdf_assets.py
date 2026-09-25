"""Regression coverage for data corrected in dexmate-urdf 0.9.0.

Source commit: d5636b5 on dexmate-urdf main (first corrected in 1b2a92f).
These values are upstream data, not estimates inferred from mirrored links.
"""
from pathlib import Path
import unittest
import xml.etree.ElementTree as ET

ASSETS = Path(__file__).resolve().parents[2] / 'crates/dexbot-model/assets/urdf/robots/humanoid'

class UpstreamAssetTests(unittest.TestCase):
    def test_corrected_inertial_blocks_match_across_variants(self):
        cases = [
            ('vega_1p', ('', '_gripper', '_f5d6'), 'back_lidar',
             '0.00423', '-0.00548 3.47734E-05 -0.00608',
             dict(ixx='6.8102e-6', ixy='0', ixz='0', iyy='4.0036e-6', iyz='0', izz='5.6834e-6')),
        ]
        for body in ('vega_1', 'vega_1p', 'vega_1u'):
            cases.append((body, ('_f5d6',), 'L_mf_l1', '0.00190', '-0.01788 0.00010 -0.00978',
                          dict(ixx='9.59615E-08', ixy='-1.38745E-12', ixz='-5.32051E-08',
                               iyy='2.34143E-07', iyz='-2.76724E-12', izz='2.28131E-07')))
        for body, variants, link, mass, origin, inertia in cases:
            for suffix in variants:
                with self.subTest(body=body, suffix=suffix, link=link):
                    root = ET.parse(ASSETS / body / f'{body}{suffix}.urdf')
                    actual = root.find(f"link[@name='{link}']/inertial")
                    self.assertIsNotNone(actual)
                    self.assertEqual(actual.find('mass').attrib, {'value': mass})
                    self.assertEqual(actual.find('origin').attrib, {'xyz': origin, 'rpy': '0 0 0'})
                    self.assertEqual(actual.find('inertia').attrib, inertia)

    def test_both_gripper_linkages_have_upstream_limits(self):
        for body in ('vega_1', 'vega_1p', 'vega_1u'):
            root = ET.parse(ASSETS / body / f'{body}_gripper.urdf')
            for side in ('L', 'R'):
                for index in (1, 2):
                    with self.subTest(body=body, side=side, index=index):
                        limit = root.find(f"joint[@name='{side}_gripper_j{index}']/limit")
                        self.assertEqual(limit.attrib, dict(lower='0', upper='0.96', effort='0', velocity='0'))
