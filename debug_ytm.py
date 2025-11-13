#!/usr/bin/env python3
"""Debug script to test YouTube Music API functionality"""

import sys
import os
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

from ytmusicapi import YTMusic
from rapidfuzz import fuzz

def test_ytm_connection():
    """Test basic YouTube Music API connection and search"""
    print("🔍 Testing YouTube Music API connection...")
    
    try:
        ytm = YTMusic()  # public mode
        print("✅ YTMusic instance created successfully")
        
        # Test with one of the seed tracks
        test_title = "Rain"
        test_artist = "Sleep Token"
        
        print(f"\n🎵 Testing search for: '{test_title}' by '{test_artist}'")
        
        # Direct song search
        query = f"{test_title} {test_artist}".strip()
        print(f"Search query: '{query}'")
        
        results = ytm.search(query, filter="songs")
        print(f"Found {len(results)} song results")
        
        if results:
            print("\n📋 First 5 results:")
            for i, result in enumerate(results[:5]):
                title = result.get("title", "N/A")
                artists = result.get("artists", [])
                artist_names = ", ".join([a.get("name", "") for a in artists]) if artists else "N/A"
                video_id = result.get("videoId", "N/A")
                
                # Calculate fuzzy match score
                result_text = f"{title} {artist_names}"
                score = fuzz.token_set_ratio(query, result_text)
                
                print(f"  {i+1}. '{title}' by '{artist_names}' (ID: {video_id}) - Score: {score}")
        else:
            print("❌ No results found!")
            
        # Test artist search
        print(f"\n🎤 Testing artist search for: '{test_artist}'")
        artist_results = ytm.search(test_artist, filter="artists")
        print(f"Found {len(artist_results)} artist results")
        
        if artist_results:
            artist = artist_results[0]
            artist_name = artist.get("artist", "N/A")
            browse_id = artist.get("browseId", "N/A")
            print(f"  Top artist: '{artist_name}' (ID: {browse_id})")
            
            if browse_id != "N/A":
                print("  Fetching artist page...")
                try:
                    artist_page = ytm.get_artist(browse_id)
                    songs = artist_page.get("songs", {}).get("results", [])
                    print(f"  Found {len(songs)} songs from artist")
                    
                    if songs:
                        print("  First 3 artist songs:")
                        for i, song in enumerate(songs[:3]):
                            song_title = song.get("title", "N/A")
                            song_id = song.get("videoId", "N/A")
                            print(f"    {i+1}. '{song_title}' (ID: {song_id})")
                except Exception as e:
                    print(f"  ❌ Error fetching artist page: {e}")
        
        return True
        
    except Exception as e:
        print(f"❌ Error: {e}")
        return False

def test_config_loading():
    """Test configuration loading"""
    print("\n⚙️  Testing configuration loading...")
    
    try:
        # Import the config loading function
        from bang_tunes import load_config, detect_root
        
        config = load_config()
        print(f"Config loaded: {config}")
        
        root = detect_root()
        print(f"Detected root: {root}")
        
        # Check if config file exists
        config_path = root / "bangtunes.toml"
        if config_path.exists():
            print(f"✅ Config file found at: {config_path}")
        else:
            print(f"⚠️  Config file not found at: {config_path}")
            
        return True
        
    except Exception as e:
        print(f"❌ Config loading error: {e}")
        return False

if __name__ == "__main__":
    print("🎵 Bang Tunes Debug Tool 🎵")
    print("=" * 40)
    
    # Test configuration
    config_ok = test_config_loading()
    
    # Test YouTube Music API
    ytm_ok = test_ytm_connection()
    
    print("\n" + "=" * 40)
    print("📊 Summary:")
    print(f"  Config loading: {'✅ OK' if config_ok else '❌ FAILED'}")
    print(f"  YouTube Music API: {'✅ OK' if ytm_ok else '❌ FAILED'}")
    
    if not ytm_ok:
        print("\n💡 Possible issues:")
        print("  - Network connectivity problems")
        print("  - YouTube Music API rate limiting")
        print("  - Missing dependencies")
        print("  - Regional restrictions")
