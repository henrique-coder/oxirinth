"""User, organization, team member, and dependency graph models."""

from __future__ import annotations

from datetime import datetime

from pydantic import AliasChoices, Field

from oxirinth.models._common import _OxirinthModel
from oxirinth.models._version import Version


__all__: list[str] = [
    "DependencyProject",
    "Organization",
    "ProjectDependencies",
    "TeamMember",
    "User",
]


class User(_OxirinthModel):
    """Public Modrinth user profile."""

    id: str
    username: str
    avatar_url: str | None = None
    raw_avatar_url: str | None = None
    bio: str | None = None
    created_at: datetime | None = Field(default=None, validation_alias=AliasChoices("created", "created_at"))
    role: str = ""
    badges: int = 0
    github_id: int | None = None


class TeamMember(_OxirinthModel):
    """Member entry belonging to a project team or organization."""

    team_id: str
    user: User
    role: str
    is_owner: bool = False
    permissions: int | None = None
    organization_permissions: int | None = None
    accepted: bool = True
    payouts_split: int | None = None
    ordering: int = 0


class Organization(_OxirinthModel):
    """Modrinth organization profile and member list."""

    id: str
    slug: str
    name: str
    team_id: str
    description: str = ""
    icon_url: str | None = None
    raw_icon_url: str | None = None
    color: int | None = None
    members: list[TeamMember] = Field(default_factory=list)


class DependencyProject(_OxirinthModel):
    """Project summary entry included in ``ProjectDependencies``."""

    id: str
    slug: str
    name: str
    summary: str = ""
    project_types: list[str] = Field(default_factory=list)
    icon_url: str | None = None
    status: str = ""
    downloads: int = 0
    followers: int = 0


class ProjectDependencies(_OxirinthModel):
    """Resolved project and version dependencies for a Modrinth project."""

    projects: list[DependencyProject] = Field(default_factory=list)
    versions: list[Version] = Field(default_factory=list)
