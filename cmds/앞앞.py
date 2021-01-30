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
            ob.sendLine('☞ 운영자 명령: [대상] 앞')
            return
        for ply in ob.channel.players:
            if ply['이름'] == line:
                if ob.env == ply.env:
                    ob.sendLine('☞ 같은 자리에요.')
                    return
                ob.clearTarget()
                ob.env.remove(ob)
                ob.env = ply.env
                ob.env.insert(ob)
                
                #if ob.enterRoom(ply.env, '소환', '소환') == False:
                #    ob.sendLine('☞ 공간이동에 실패하였습니다.')
                
                return
        ob.sendLine('☞ 활동중인 그런 무림인이 없어요. ^^')
        return
        
        

