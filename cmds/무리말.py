# -*- coding: utf-8 -*-

from objs.cmd import Command

class CmdObj(Command):
                
    def cmd(self, ob, line):
        if len(line) == 0:
            ob.sendLine('☞ 사용법: [내용] 무리말(;)')
            return
        if len(line) > 160:
            ob.sendLine('☞ 너무 길어요. ^^')
            return
        if ob.env == None:
            return
        if ob.env.noComm():
            ob.sendLine('☞ 이지역에서는 어떠한 통신도 불가능합니다.')
            return
        if ob.Party == None:
            ob.sendLine('☞ 당신이 속한 무리가 없어요. ^^')
            return
        msg=''
        if ob.Party == ob:
            msg= '[1m[40m[32m◀[0m[40m[37m%s[1m[40m[32m▶[0m[40m[37m ' % ob['이름']
            #msg = '[[1m[36m무리말[0m[37m] %s : %s' % (ob['이름'], msg)

        else:
            msg= '[1m[40m[32m◁[0m[40m[37m%s[1m[40m[32m▷[0m[40m[37m ' % ob['이름']
            #msg = '[[1m[33m무리말[0m[37m] %s : %s' % (ob['이름'], msg)
        msg +=line
        ob.sendToParty(msg, prompt = True)
