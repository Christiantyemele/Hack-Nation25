"""User model and related database operations."""

from pydantic import BaseModel, EmailStr, Field
from typing import Optional


class User(BaseModel):
    """User model."""
    id: int
    username: str
    email: EmailStr
    hashed_password: str
    is_active: bool = True

    class Config:
        orm_mode = True
