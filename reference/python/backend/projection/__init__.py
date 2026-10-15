"""GRID-powered fantasy projection (roadmap Phase 2b).

Projects a stat line (volume x efficiency) then scores it through any format:
the volume/role model lives here, the GRID talent features feed it, and the
fitted stat-line model combines them.  See the per-module docstrings.
"""
from __future__ import annotations

from backend.projection.features import TalentFeatures, assemble_talent_features
from backend.projection.model import RateModel, StatLineModel, fit_stat_line_model
from backend.projection.preseason import project_preseason
from backend.projection.sv_to_points import SVToPointsMap, fit_sv_to_points
from backend.projection.volume import VOLUME_COLS, VolumeProjection, project_volume

__all__ = [
    "VOLUME_COLS", "VolumeProjection", "project_volume",
    "TalentFeatures", "assemble_talent_features",
    "RateModel", "StatLineModel", "fit_stat_line_model",
    "SVToPointsMap", "fit_sv_to_points",
    "project_preseason",
]
