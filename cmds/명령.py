# -*- coding: utf-8 -*-

from objs.cmd import Command
from include.ansi import *

class CmdObj(Command):
    level = 2000
    def cmd(self, ob, line):
        if getInt(ob['관리자등급']) < 2000:
            ob.sendLine('☞ 무슨 말인지 모르겠어요. *^_^*')
            return
        words = line.split(None, 1)
        if line == '' or len(words) < 2:
            ob.sendLine('☞ 운영자 명령: [대상] [내용] 명령')
            return
        
        obj = ob.env.findObjName(words[0])
        if obj == None or is_player(obj) == False:
            ob.sendLine('☞ 그런 대상이 없어요. *^_^*')
            return
        obj.sendLine('')
        obj.do_command(words[1])
        
        

