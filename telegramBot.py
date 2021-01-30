# -*- coding: utf-8 -*-

import telebot
import Queue
import threading
import time
from objs.player import Player
from include.define import ACTIVE

bot = telebot.TeleBot("1161166098:AAFnGD4oqW_nERu5TTObYycYmENOjlnnOGQ")

@bot.message_handler(commands=['start', 'help'])
def send_welcome(message):
        print("Howdy, how are you doing?")
        bot.reply_to(message, "Howdy, how are you doing?")

@bot.message_handler(func=lambda message: True)
def echo_all(message):
        #print(message)
        type = '[1;34m電報[0;37m'
        who = ''
        first = message.from_user.first_name
        if first != None:
            first_enc = first.encode('euc-kr', 'ignore')
            who = first_enc
        last = message.from_user.last_name
        if last != None:
            last_enc = last.encode('euc-kr', 'ignore')
            who += ' ' + last_enc
        text = message.text.encode('euc-kr', 'ignore')
        timemsg = time.strftime('[%H:%M] ', time.localtime())
        msg = who + '(%s) : %s' % (type, text)

        #m1 = self.ANSI(msg, True)
        #m2 = self.ANSI(msg, False)
        m1 = m2 = msg

        Player.chatHistory.append(timemsg + m1 + '[0;37m')
        if len(Player.chatHistory) > 24:
            Player.chatHistory.__delitem__(0)

        # 잡담 로그를 파일로!!!
        from client import Client
        for ply in Client.players:
            if ply.state != ACTIVE:
                continue
            if ply.checkConfig('외침거부'):
                continue

            if ply.checkConfig('잡담시간보기'):
                if ply.checkConfig('사용자안시거부'):
                    buf = timemsg + m2
                else:
                    buf = timemsg + m1
            else:
                if ply.checkConfig('사용자안시거부'):
                    buf = m2
                else:
                    buf = m1

            ply.sendLine('\r\n' + buf + '[0;37;40m')
            ply.lpPrompt()
        #bot.reply_to(message, message.text)

#bot.polling()

#queue = Queue.Queue()

class TelegramPostThread(threading.Thread):
    def __init__(self, queue):
        threading.Thread.__init__(self)
        self.queue = queue

    def run(self):
        while True:
            msg = self.queue.get()
            try:
                bot.send_message(-1001313269219, msg)
            except:
                pass
            
            self.queue.task_done()

class TelegramPollingThread(threading.Thread):
    def __init__(self, queue):
        threading.Thread.__init__(self)

    def run(self):
        while True:
            bot.polling()
