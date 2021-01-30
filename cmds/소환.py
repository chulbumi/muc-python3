# -*- coding: utf-8 -*-

from objs.cmd import Command
from include.ansi import *

class CmdObj(Command):
    level = 2000
    def cmd(self, ob, line):
        if getInt(ob['관리자등급']) < 2000:
            ob.sendLine('☞ 무슨 말인지 모르겠어요. *^_^*')
            return
        if line == '':
            ob.sendLine('☞ 운영자 명령: [대상] 소환')
            return
        for ply in ob.channel.players:
            if ply['이름'] == line:
                if ply.env == ob.env:
                    ob.sendLine('☞ 같은 곳이에요. ^^')
                    return
                if ply.enterRoom(ob.env, '소환', '소환') == False:
                    ob.sendLine('☞ 공간이동에 실패하였습니다.')
                ply.clearTarget()
                ply.lpPrompt()
                return
        ob.sendLine('☞ 활동중인 그런 무림인이 없어요. ^^')
        return
        
        

