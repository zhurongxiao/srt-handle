from youtube_transcript_api import YouTubeTranscriptApi

transcript = YouTubeTranscriptApi.get_transcript("UF8uR6Z6KLc")
for line in transcript:
    print(line['text'])
