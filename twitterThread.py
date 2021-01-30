# -*- coding: utf-8 -*-

import twitter
import Queue
import threading
import time
from objs.player import Player
from include.define import ACTIVE

queue = Queue.Queue()
api = twitter.Api(
consumer_key='tGOOJ1Zxn1Vtm28Tv48NOA',
consumer_secret='JUTtVmstjev4t41HxuW5HzExaWj8Lcuh0mqQIiv2k',
access_token_key='452262168-oQ15XADL3u9SNTV3pMzRUNabz961Uw8C57fYscHG',
access_token_secret='561wrUnozc7Z9eNZfxeTITOEUvI8GtViaos79lxirWc')
last_twitter_id = None

class PostThread(threading.Thread):
    def __init__(self, queue):
        threading.Thread.__init__(self)
        self.queue = queue

    def run(self):

        while True:
            msg = self.queue.get()
            try:
                status = api.PostUpdate(msg)
            except:
                pass
            
            self.queue.task_done()

class GetTwitterThread(threading.Thread):
    def __init__(self, queue):
        threading.Thread.__init__(self)

    def run(self):
        msgs = api.GetDirectMessages()
        if len(msgs) != 0:
            last_twitter_id = msgs[0].id
        while True:
            try:
                msgs = api.GetDirectMessages(None, last_twitter_id)
            except:
                pass
            if len(msgs) != 0:
                last_twitter_id = msgs[0].id
            for msg in msgs:
                _content = msg.text.encode('euc-kr', 'ignore')
                timemsg = time.strftime('[%H:%M] ', time.localtime())
                buf = '[1m' + msg.sender_screen_name.encode('euc-kr', 'ignore') +'[0;37m'
                buf += '([1;34mtwitter[0;37m) : %s' % _content 
                Player.chatHistory.append(timemsg + buf)
                if len(Player.chatHistory) > 24:
                    Player.chatHistory.__delitem__(0)

                from client import Client 
                for ply in Client.players:
                    if ply.state != ACTIVE:
                        continue
                    if ply.checkConfig('외침거부'):
                        continue
                    if ply.checkConfig('잡담시간보기'):
                        ply.sendLine('\r\n' + timemsg + buf + '[0;37;40m')
                    else:
                        ply.sendLine('\r\n' + buf + '[0;37;40m')
                    ply.lpPrompt()

            time.sleep(15)
