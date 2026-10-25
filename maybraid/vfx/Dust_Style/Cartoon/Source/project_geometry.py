"""Native project callbacks; all resources resolve within this category."""
from pathlib import Path
import json,hou,numpy as np
import houdini_geometry as geo
ROOT=Path(__file__).resolve().parents[1]
def cartoon(t,effect):return geo.surface(t,effect)
def backdrop():return geo.stage()
