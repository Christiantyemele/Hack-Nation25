"""User model for authentication and authorization."""

from datetime import datetime
from typing import Optional

from pydantic import BaseModel, EmailStr
from sqlalchemy import Boolean, Column, DateTime, Integer, String
from sqlalchemy.ext.declarative import declarative_base

Base = declarative_base()


class UserTable(Base):
    """User database table."""
    
    __tablename__ = "users"
    
    id = Column(Integer, primary_key=True, index=True)
    username = Column(String(50), unique=True, index=True, nullable=False)
    email = Column(String(100), unique=True, index=True, nullable=False)
    hashed_password = Column(String(255), nullable=False)
    is_active = Column(Boolean, default=True, nullable=False)
    is_superuser = Column(Boolean, default=False, nullable=False)
    created_at = Column(DateTime, default=datetime.utcnow, nullable=False)
    updated_at = Column(DateTime, default=datetime.utcnow, onupdate=datetime.utcnow, nullable=False)


class User(BaseModel):
    """User Pydantic model for API responses."""
    
    id: int
    username: str
    email: EmailStr
    is_active: bool = True
    is_superuser: bool = False
    created_at: Optional[datetime] = None
    updated_at: Optional[datetime] = None
    
    # Exclude hashed_password from API responses
    hashed_password: Optional[str] = None
    
    class Config:
        """Pydantic configuration."""
        from_attributes = True
        # Exclude hashed_password from serialization
        fields = {"hashed_password": {"write_only": True}}


class UserCreate(BaseModel):
    """User creation model."""
    
    username: str
    email: EmailStr
    password: str
    is_active: bool = True
    is_superuser: bool = False


class UserUpdate(BaseModel):
    """User update model."""
    
    username: Optional[str] = None
    email: Optional[EmailStr] = None
    password: Optional[str] = None
    is_active: Optional[bool] = None
    is_superuser: Optional[bool] = None


class UserInDB(User):
    """User model with hashed password for internal use."""
    
    hashed_password: str