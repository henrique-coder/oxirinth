"""Read-only user, organization, team member, and dependency graph models."""

from __future__ import annotations

from datetime import datetime

from pydantic import Field

from oxirinth.models.common import OxirinthModel
from oxirinth.models.version import Version


__all__: list[str] = [
    "DependencyProject",
    "Organization",
    "ProjectDependencies",
    "TeamMember",
    "User",
]


class User(OxirinthModel):
    """Public Modrinth user profile returned by ``/v3/user/{id_or_username}``."""

    id: str
    username: str
    avatar_url: str | None = None
    raw_avatar_url: str | None = None
    bio: str | None = None
    created: datetime | None = None
    role: str = ""
    badges: int = 0
    github_id: int | None = None


class TeamMember(OxirinthModel):
    """Member entry for a project team or organization."""

    team_id: str
    user: User
    role: str
    is_owner: bool = False
    permissions: int | None = None
    organization_permissions: int | None = None
    accepted: bool = True
    payouts_split: int | None = None
    ordering: int = 0


class Organization(OxirinthModel):
    """Modrinth organization profile returned by ``/v3/organization/{id_or_slug}``."""

    id: str
    slug: str
    name: str
    team_id: str
    description: str = ""
    icon_url: str | None = None
    raw_icon_url: str | None = None
    color: int | None = None
    members: list[TeamMember] = Field(default_factory=list)


class DependencyProject(OxirinthModel):
    """Project summary entry returned inside ``/v3/project/{id}/dependencies``."""

    id: str
    slug: str
    name: str
    summary: str = ""
    project_types: list[str] = Field(default_factory=list)
    icon_url: str | None = None
    status: str = ""
    downloads: int = 0
    followers: int = 0


class ProjectDependencies(OxirinthModel):
    """Resolved project and version dependencies for a Modrinth project."""

    projects: list[DependencyProject] = Field(default_factory=list)
    versions: list[Version] = Field(default_factory=list)
